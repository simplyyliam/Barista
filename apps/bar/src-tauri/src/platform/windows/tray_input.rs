//! Win32 side of tray clicks (ADR 0027): posting the app's own callback message, the double-click
//! time, and turning a screen point into the owner window's coordinates. The run of messages and
//! their encoding are plain Rust in `tray/click.rs`.

use super::tray_host::from_32;
use windows::Win32::Foundation::{LPARAM, POINT, WPARAM};
use crate::tray::click::{to_owner, Awareness};
use windows::Win32::Graphics::Gdi::{MonitorFromPoint, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::HiDpi::{
    GetAwarenessFromDpiAwarenessContext, GetDpiForMonitor, GetDpiForSystem,
    GetWindowDpiAwarenessContext, DPI_AWARENESS_PER_MONITOR_AWARE, DPI_AWARENESS_SYSTEM_AWARE,
    DPI_AWARENESS_UNAWARE, MDT_EFFECTIVE_DPI,
};
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

/// A physical screen point in the coordinates the owner window lives in (see `click::to_owner`).
/// (`PhysicalToLogicalPointForPerMonitorDPI` fails for these windows, so we convert ourselves.)
/// Falls back to the physical point when Windows can't tell.
pub fn to_owner_coordinates(hwnd: u32, point: (i32, i32)) -> (i32, i32) {
    // SAFETY: plain queries; a stale window handle just gives an unknown awareness.
    unsafe {
        let context = GetWindowDpiAwarenessContext(from_32(hwnd));
        let awareness = match GetAwarenessFromDpiAwarenessContext(context) {
            DPI_AWARENESS_UNAWARE => Awareness::Unaware,
            DPI_AWARENESS_SYSTEM_AWARE => Awareness::System,
            DPI_AWARENESS_PER_MONITOR_AWARE => Awareness::PerMonitor,
            _ => return point,
        };
        if awareness == Awareness::PerMonitor {
            return point;
        }
        let monitor = MonitorFromPoint(POINT { x: point.0, y: point.1 }, MONITOR_DEFAULTTONEAREST);
        let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
        let monitor_dpi = match GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) {
            Ok(()) if dpi_x > 0 => dpi_x,
            _ => return point,
        };
        to_owner(point, awareness, monitor_dpi, GetDpiForSystem())
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
