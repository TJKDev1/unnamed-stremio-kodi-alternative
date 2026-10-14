//! Spike S1: libmpv under a transparent webview (Tauri v2). Throwaway code (D65).
//!
//! usage: s1-shell [--file PATH] [--window transparent|opaque] [--embed|--no-embed]
//!                 [--hint-mode source|target] [--no-hint] [--auto bench|stress] [--seconds N]
//!                 [--mpv key=value]...
//!
//! See spikes/s1-shell/README.md.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod mpv;
#[cfg(windows)]
mod win;

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use serde::Serialize;
use tauri::{Emitter, LogicalSize, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[derive(Debug, Clone, Serialize)]
struct Args {
    file: Option<String>,
    /// "transparent": Tauri transparent window (Rillio). "opaque": normal window, webview
    /// background alpha 0 (stremio-shell-ng).
    window: String,
    embed: bool,
    hint: bool,
    hint_mode: String,
    auto: Option<String>,
    seconds: u64,
    /// Extra mpv options (`--mpv key=value`, repeatable), applied last.
    mpv: Vec<String>,
}

fn parse_args() -> Args {
    let mut a = Args {
        file: None,
        window: "transparent".into(),
        // Only Windows embeds; elsewhere the fallback is the plan until tested on hardware (D64).
        embed: cfg!(windows),
        hint: true,
        hint_mode: "source".into(),
        auto: None,
        seconds: 10,
        mpv: Vec::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--file" => a.file = it.next(),
            "--window" => a.window = it.next().expect("--window transparent|opaque"),
            "--embed" => a.embed = true,
            "--no-embed" => a.embed = false,
            "--no-hint" => a.hint = false,
            "--hint-mode" => a.hint_mode = it.next().expect("--hint-mode source|target"),
            "--auto" => a.auto = it.next(),
            "--mpv" => a.mpv.push(it.next().expect("--mpv key=value")),
            "--seconds" => a.seconds = it.next().and_then(|s| s.parse().ok()).expect("--seconds N"),
            other => panic!("unknown argument {other}"),
        }
    }
    a
}

struct AppState {
    args: Args,
    mpv: Option<mpv::Shared>,
    hwnd: isize,
    results: PathBuf,
    started: String,
}

fn results_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.cache/s1-results");
    fs::create_dir_all(&dir).expect("create results dir");
    dir
}

fn timestamp() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

fn window_handle(w: &WebviewWindow) -> isize {
    match w.window_handle().expect("window handle").as_raw() {
        RawWindowHandle::Win32(h) => h.hwnd.get(),
        RawWindowHandle::AppKit(h) => h.ns_view.as_ptr() as isize,
        RawWindowHandle::Xlib(h) => h.window as isize,
        _ => 0,
    }
}

fn start_mpv(
    args: &Args,
    wid: isize,
    log_path: PathBuf,
    window: WebviewWindow,
) -> Result<mpv::Shared, String> {
    let m = mpv::Mpv::load(&mpv::default_path())?;
    if args.embed && wid != 0 {
        m.set_option("wid", &wid.to_string())?;
    } else {
        // Fallback under test: mpv in its own borderless window.
        m.set_option("force-window", "yes")?;
        m.set_option("border", "no")?;
    }
    for (k, v) in [
        ("vo", "gpu-next"),
        ("hwdec", "auto-safe"),
        ("keep-open", "yes"),
        ("idle", "yes"),
        ("loop-file", "inf"),
        ("osc", "no"),
        ("osd-level", "0"),
        // The webview owns input (stremio-shell-ng / Rillio do the same).
        ("input-default-bindings", "no"),
        ("input-vo-keyboard", "no"),
        ("input-cursor", "no"),
        ("cursor-autohide", "no"),
        ("terminal", "no"),
    ] {
        m.set_option(k, v)?;
    }
    if cfg!(windows) {
        m.set_option("gpu-context", "d3d11")?;
    }
    m.set_option(
        "target-colorspace-hint",
        if args.hint { "auto" } else { "no" },
    )?;
    m.set_option("target-colorspace-hint-mode", &args.hint_mode)?;
    for kv in &args.mpv {
        let (k, v) = kv
            .split_once('=')
            .ok_or_else(|| format!("--mpv {kv}: expected key=value"))?;
        m.set_option(k, v)?;
    }
    m.initialize("v")?;
    let m = Arc::new(m);

    let ev = m.clone();
    std::thread::spawn(move || {
        let mut log = std::io::BufWriter::new(fs::File::create(&log_path).expect("mpv log"));
        let t0 = Instant::now();
        loop {
            let Some(e) = ev.wait_event(-1.0) else {
                continue;
            };
            match e {
                mpv::Event::Log {
                    prefix,
                    level,
                    text,
                } => {
                    let _ = write!(
                        log,
                        "[{:8.3}][{level}][{prefix}] {text}",
                        t0.elapsed().as_secs_f64()
                    );
                }
                mpv::Event::Shutdown => break,
                other => {
                    let _ = writeln!(log, "[{:8.3}] event {other:?}", t0.elapsed().as_secs_f64());
                    if matches!(other, mpv::Event::VideoReconfig | mpv::Event::FileLoaded) {
                        #[cfg(windows)]
                        if wid != 0 {
                            let moved = win::push_mpv_to_bottom(wid);
                            let _ = writeln!(
                                log,
                                "[{:8.3}] pushed {moved} mpv window(s) to bottom",
                                t0.elapsed().as_secs_f64()
                            );
                        }
                    }
                    let _ = window.emit("mpv-event", format!("{other:?}"));
                }
            }
        }
        let _ = log.flush();
    });

    if let Some(f) = &args.file {
        m.command(&["loadfile", f])?;
    }
    Ok(m)
}

fn mpv_of(state: &tauri::State<'_, AppState>) -> Result<mpv::Shared, String> {
    state.mpv.clone().ok_or_else(|| "mpv not running".into())
}

#[tauri::command]
fn config(state: tauri::State<'_, AppState>) -> Args {
    state.args.clone()
}

#[tauri::command]
fn mpv_command(state: tauri::State<'_, AppState>, args: Vec<String>) -> Result<(), String> {
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    mpv_of(&state)?.command(&refs)
}

#[tauri::command]
fn mpv_set(state: tauri::State<'_, AppState>, name: String, value: String) -> Result<(), String> {
    mpv_of(&state)?.set_property(&name, &value)
}

#[tauri::command]
fn mpv_get(
    state: tauri::State<'_, AppState>,
    names: Vec<String>,
) -> Result<BTreeMap<String, Option<String>>, String> {
    let m = mpv_of(&state)?;
    Ok(names
        .into_iter()
        .map(|n| {
            let v = m.get_property(&n);
            (n, v)
        })
        .collect())
}

#[tauri::command]
fn child_windows(state: tauri::State<'_, AppState>) -> Vec<String> {
    #[cfg(windows)]
    return win::describe_children(state.hwnd);
    #[cfg(not(windows))]
    return vec![format!(
        "native handle {:#x} (child windows: Windows only)",
        state.hwnd
    )];
}

#[tauri::command]
fn list_media() -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.cache/s1-media");
    let mut out: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| !p.to_string_lossy().contains(".partial."))
        .map(|p| p.canonicalize().unwrap_or(p).to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

/// Drives the window from Rust while the page measures its own frame times.
#[tauri::command]
async fn stress(window: WebviewWindow, kind: String, seconds: u64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let e = |r: tauri::Result<()>| r.map_err(|e| e.to_string());
        let end = Instant::now() + Duration::from_secs(seconds);
        match kind.as_str() {
            "resize" => {
                // Sweep the size back and forth, one step per ~frame, like a user dragging the corner.
                let (mut w, mut dir) = (960.0, 1.0);
                while Instant::now() < end {
                    w += dir * 16.0;
                    if !(960.0..=1600.0).contains(&w) {
                        dir = -dir;
                    }
                    e(window.set_size(LogicalSize::new(w, w * 9.0 / 16.0)))?;
                    std::thread::sleep(Duration::from_millis(16));
                }
                e(window.set_size(LogicalSize::new(1280.0, 720.0)))
            }
            "fullscreen" => {
                while Instant::now() < end {
                    let fs = window.is_fullscreen().map_err(|e| e.to_string())?;
                    e(window.set_fullscreen(!fs))?;
                    std::thread::sleep(Duration::from_millis(1500));
                }
                e(window.set_fullscreen(false))
            }
            "minimize" => {
                while Instant::now() < end {
                    e(window.minimize())?;
                    std::thread::sleep(Duration::from_millis(800));
                    e(window.unminimize())?;
                    std::thread::sleep(Duration::from_millis(1200));
                }
                Ok(())
            }
            other => Err(format!("unknown stress kind {other}")),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn save_result(
    state: tauri::State<'_, AppState>,
    name: String,
    json: String,
) -> Result<String, String> {
    let path = state.results.join(format!("{}-{name}.json", state.started));
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

/// Appends a line from the page to <started>-page.log (survives a hung page).
#[tauri::command]
fn page_log(state: tauri::State<'_, AppState>, line: String) {
    let path = state.results.join(format!("{}-page.log", state.started));
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}

#[tauri::command]
fn quit(app: tauri::AppHandle, state: tauri::State<'_, AppState>) {
    if let Some(m) = &state.mpv {
        let _ = m.command(&["quit"]);
    }
    app.exit(0);
}

fn main() {
    let args = parse_args();
    // Avoids the white flash before the transparent background is applied (WebView2 reads this
    // when it starts; AARRGGBB).
    // SAFETY: set before any other thread exists.
    unsafe { std::env::set_var("WEBVIEW2_DEFAULT_BACKGROUND_COLOR", "00000000") };

    tauri::Builder::default()
        .setup(move |app| {
            let mut b =
                WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                    .title("S1 shell")
                    .inner_size(1280.0, 720.0);
            b = match args.window.as_str() {
                "transparent" => b.transparent(true),
                "opaque" => b.background_color(tauri::window::Color(0, 0, 0, 0)),
                other => panic!("unknown --window {other}"),
            };
            let window = b.build()?;
            let hwnd = window_handle(&window);
            let results = results_dir();
            let started = timestamp();
            let log = results.join(format!("{started}-mpv.log"));
            let mpv = match start_mpv(&args, hwnd, log, window.clone()) {
                Ok(m) => Some(m),
                Err(e) => {
                    eprintln!("mpv failed to start: {e}");
                    None
                }
            };
            app.manage(AppState {
                args: args.clone(),
                mpv,
                hwnd,
                results,
                started,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config,
            mpv_command,
            mpv_set,
            mpv_get,
            child_windows,
            list_media,
            stress,
            save_result,
            page_log,
            quit
        ])
        .run(tauri::generate_context!())
        .expect("tauri run");
}
