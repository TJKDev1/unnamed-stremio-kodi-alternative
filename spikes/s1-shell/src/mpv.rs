//! Minimal libmpv client API, loaded at run time with `libloading` so the spike builds (and CI
//! checks it) without libmpv present. Only the calls the spike needs.

use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::path::Path;
use std::sync::Arc;

type Handle = *mut c_void;

#[repr(C)]
struct RawEvent {
    event_id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

#[repr(C)]
struct RawLogMessage {
    prefix: *const c_char,
    level: *const c_char,
    text: *const c_char,
    log_level: c_int,
}

// Payloads are only read through Debug (logged and forwarded to the page).
#[allow(dead_code)]
#[derive(Debug)]
pub enum Event {
    Shutdown,
    Log {
        prefix: String,
        level: String,
        text: String,
    },
    StartFile,
    EndFile,
    FileLoaded,
    VideoReconfig,
    PlaybackRestart,
    Other(i32),
}

struct Api {
    _lib: libloading::Library,
    create: unsafe extern "C" fn() -> Handle,
    initialize: unsafe extern "C" fn(Handle) -> c_int,
    terminate_destroy: unsafe extern "C" fn(Handle),
    set_option_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    set_property_string: unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int,
    get_property_string: unsafe extern "C" fn(Handle, *const c_char) -> *mut c_char,
    command: unsafe extern "C" fn(Handle, *const *const c_char) -> c_int,
    wait_event: unsafe extern "C" fn(Handle, c_double) -> *mut RawEvent,
    request_log_messages: unsafe extern "C" fn(Handle, *const c_char) -> c_int,
    error_string: unsafe extern "C" fn(c_int) -> *const c_char,
    free: unsafe extern "C" fn(*mut c_void),
}

/// One mpv instance. libmpv's API is thread-safe except `wait_event`, which only the event
/// thread calls.
pub struct Mpv {
    api: Api,
    handle: Handle,
}

// SAFETY: libmpv documents every client API function as thread-safe (except mpv_wait_event,
// which `events()` keeps on one thread by convention in this spike).
unsafe impl Send for Mpv {}
unsafe impl Sync for Mpv {}

fn cstr(s: &str) -> CString {
    CString::new(s).expect("no NUL in mpv strings")
}

impl Mpv {
    /// Loads libmpv from `path` and creates an uninitialized instance.
    pub fn load(path: &Path) -> Result<Self, String> {
        // SAFETY: loading a C library and resolving symbols with their documented signatures.
        unsafe {
            let lib = libloading::Library::new(path)
                .map_err(|e| format!("load {}: {e}", path.display()))?;
            macro_rules! sym {
                ($name:literal) => {
                    *lib.get(concat!($name, "\0").as_bytes())
                        .map_err(|e| format!("{}: {e}", $name))?
                };
            }
            let api = Api {
                create: sym!("mpv_create"),
                initialize: sym!("mpv_initialize"),
                terminate_destroy: sym!("mpv_terminate_destroy"),
                set_option_string: sym!("mpv_set_option_string"),
                set_property_string: sym!("mpv_set_property_string"),
                get_property_string: sym!("mpv_get_property_string"),
                command: sym!("mpv_command"),
                wait_event: sym!("mpv_wait_event"),
                request_log_messages: sym!("mpv_request_log_messages"),
                error_string: sym!("mpv_error_string"),
                free: sym!("mpv_free"),
                _lib: lib,
            };
            let handle = (api.create)();
            if handle.is_null() {
                return Err("mpv_create returned NULL".into());
            }
            Ok(Self { api, handle })
        }
    }

    fn check(&self, what: &str, rc: c_int) -> Result<(), String> {
        if rc >= 0 {
            return Ok(());
        }
        // SAFETY: mpv_error_string returns a static string for any code.
        let msg = unsafe { CStr::from_ptr((self.api.error_string)(rc)) }.to_string_lossy();
        Err(format!("{what}: {msg}"))
    }

    pub fn set_option(&self, name: &str, value: &str) -> Result<(), String> {
        let (n, v) = (cstr(name), cstr(value));
        // SAFETY: valid handle and NUL-terminated strings.
        let rc = unsafe { (self.api.set_option_string)(self.handle, n.as_ptr(), v.as_ptr()) };
        self.check(&format!("option {name}={value}"), rc)
    }

    pub fn initialize(&self, log_level: &str) -> Result<(), String> {
        let level = cstr(log_level);
        // SAFETY: valid handle.
        unsafe {
            self.check("mpv_initialize", (self.api.initialize)(self.handle))?;
            self.check(
                "log messages",
                (self.api.request_log_messages)(self.handle, level.as_ptr()),
            )
        }
    }

    pub fn set_property(&self, name: &str, value: &str) -> Result<(), String> {
        let (n, v) = (cstr(name), cstr(value));
        // SAFETY: valid handle and NUL-terminated strings.
        let rc = unsafe { (self.api.set_property_string)(self.handle, n.as_ptr(), v.as_ptr()) };
        self.check(&format!("set {name}"), rc)
    }

    /// The property as mpv's string form (maps and lists come out as JSON), or None if unavailable.
    pub fn get_property(&self, name: &str) -> Option<String> {
        let n = cstr(name);
        // SAFETY: valid handle; the returned string is owned by us and freed with mpv_free.
        unsafe {
            let p = (self.api.get_property_string)(self.handle, n.as_ptr());
            if p.is_null() {
                return None;
            }
            let s = CStr::from_ptr(p).to_string_lossy().into_owned();
            (self.api.free)(p.cast());
            Some(s)
        }
    }

    pub fn command(&self, args: &[&str]) -> Result<(), String> {
        let owned: Vec<CString> = args.iter().map(|a| cstr(a)).collect();
        let mut ptrs: Vec<*const c_char> = owned.iter().map(|a| a.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        // SAFETY: NULL-terminated array of NUL-terminated strings that outlive the call.
        let rc = unsafe { (self.api.command)(self.handle, ptrs.as_ptr()) };
        self.check(&format!("command {args:?}"), rc)
    }

    /// Blocks for the next event. Call from one thread only.
    pub fn wait_event(&self, timeout: f64) -> Option<Event> {
        // SAFETY: valid handle; the event and its data stay valid until the next wait_event.
        unsafe {
            let ev = &*(self.api.wait_event)(self.handle, timeout);
            let s = |p: *const c_char| {
                if p.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(p).to_string_lossy().into_owned()
                }
            };
            Some(match ev.event_id {
                0 => return None,
                1 => Event::Shutdown,
                2 => {
                    let m = &*(ev.data as *const RawLogMessage);
                    let _ = m.log_level;
                    Event::Log {
                        prefix: s(m.prefix),
                        level: s(m.level),
                        text: s(m.text),
                    }
                }
                6 => Event::StartFile,
                7 => Event::EndFile,
                8 => Event::FileLoaded,
                17 => Event::VideoReconfig,
                21 => Event::PlaybackRestart,
                id => Event::Other(id),
            })
        }
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        // SAFETY: the handle is not used after this.
        unsafe { (self.api.terminate_destroy)(self.handle) }
    }
}

/// Where to find libmpv: `S1_LIBMPV`, else the copy fetched by tools/fetch-libmpv.mjs (Windows),
/// else the system library name.
pub fn default_path() -> std::path::PathBuf {
    if let Some(p) = std::env::var_os("S1_LIBMPV") {
        return p.into();
    }
    if cfg!(windows) {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cache/libmpv/libmpv-2.dll")
    } else if cfg!(target_os = "macos") {
        "libmpv.2.dylib".into()
    } else {
        "libmpv.so.2".into()
    }
}

pub type Shared = Arc<Mpv>;
