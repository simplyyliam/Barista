//! Win32 side of tray clicks (ADR 0027): posting the app's own callback message and the double-click
//! time. The run of messages and
//! their encoding are plain Rust in `tray/click.rs`.

use super::tray_host::from_32;
use windows::Win32::Foundation::{LPARAM, POINT, WPARAM};
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
