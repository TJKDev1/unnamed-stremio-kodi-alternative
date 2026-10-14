//! Win32 helpers: keep mpv's child window below the webview, and describe the child windows
//! (class + z-order) for the report.

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GW_CHILD, GW_HWNDNEXT, GetClassNameW, GetWindow, GetWindowRect, HWND_BOTTOM, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
};

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    // SAFETY: buffer length matches.
    let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

/// Direct children of `parent`, in z-order (top first).
fn direct_children(parent: HWND) -> Vec<HWND> {
    let mut out = Vec::new();
    // SAFETY: plain window queries.
    let mut h = unsafe { GetWindow(parent, GW_CHILD) };
    while !h.is_null() {
        out.push(h);
        h = unsafe { GetWindow(h, GW_HWNDNEXT) };
    }
    out
}

/// Moves every direct child whose class is "mpv" to the bottom of the z-order, so the webview
/// stays on top. Returns how many it moved. mpv recreates its window on VO re-init, so call this
/// after every video reconfig.
pub fn push_mpv_to_bottom(parent: isize) -> usize {
    let parent = parent as HWND;
    let mut moved = 0;
    for h in direct_children(parent) {
        if class_name(h) == "mpv" {
            // SAFETY: valid child window handle.
            unsafe {
                SetWindowPos(
                    h,
                    HWND_BOTTOM,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            moved += 1;
        }
    }
    moved
}

/// "class WxH" for each direct child, top of the z-order first.
pub fn describe_children(parent: isize) -> Vec<String> {
    direct_children(parent as HWND)
        .into_iter()
        .map(|h| {
            let mut r = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            // SAFETY: valid window handle and out-pointer.
            unsafe { GetWindowRect(h, &mut r) };
            format!(
                "{} {}x{}",
                class_name(h),
                r.right - r.left,
                r.bottom - r.top
            )
        })
        .collect()
}
