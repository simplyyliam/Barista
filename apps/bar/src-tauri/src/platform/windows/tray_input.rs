//! Win32 side of tray clicks (ADR 0027): posting the app's own callback message, the double-click
//! time, and turning a screen point into the owner window's coordinates. The run of messages and
//! their encoding are plain Rust in `tray/click.rs`.

use super::tray_host::from_32;
use windows::Win32::Foundation::{LPARAM, POINT, WPARAM};
use windows::Win32::UI::HiDpi::PhysicalToLogicalPointForPerMonitorDPI;
use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, PostMessageW};

/// How close two clicks must be to count as a double click (the user's Windows setting).
pub fn double_click_ms() -> u64 {
    // SAFETY: plain call, no arguments.
    u64::from(unsafe { GetDoubleClickTime() })
}

/// The mouse pointer in screen pixels.
pub fn cursor() -> Option<(i32, i32)> {
    let mut p = POINT::default();
    // SAFETY: GetCursorPos only writes into our local POINT.
    unsafe { GetCursorPos(&mut p) }.ok().map(|()| (p.x, p.y))
}

/// A physical screen point in the coordinates the owner window lives in. An app that is not DPI
/// aware sees scaled pixels, so a version 4 anchor must be scaled the same way (rule 8: physical at
/// the boundary, converted once, here). Falls back to the physical point.
pub fn to_owner_coordinates(hwnd: u32, (x, y): (i32, i32)) -> (i32, i32) {
    let mut p = POINT { x, y };
    // SAFETY: only reads and writes our local POINT; a stale window just fails.
    if unsafe { PhysicalToLogicalPointForPerMonitorDPI(Some(from_32(hwnd)), &mut p) }.as_bool() {
        (p.x, p.y)
    } else {
        (x, y)
    }
}

/// Posts one callback message to the icon's window. `PostMessage` never waits, so a hung app
/// cannot freeze the bar (rule 3). False when the window is gone or its queue is full.
pub fn post(hwnd: u32, callback_message: u32, wparam: usize, lparam: isize) -> bool {
    // SAFETY: plain call; a stale handle just fails.
    unsafe {
        PostMessageW(
            Some(from_32(hwnd)),
            callback_message,
            WPARAM(wparam),
            LPARAM(lparam),
        )
    }
    .is_ok()
}
