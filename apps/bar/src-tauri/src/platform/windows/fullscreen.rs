//! Spotting a full-screen app (game, video, F11 browser) in front, so the bar can step aside.
//! Event-driven: Windows tells us when the foreground window changes, no polling.

use crate::bar::{covers, Rect};
use std::sync::OnceLock;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowRect, IsIconic, IsWindowVisible,
    EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};

static HANDLER: OnceLock<fn()> = OnceLock::new();

/// Calls `on_change` (on the UI thread) every time another window comes to the front.
/// Must be called from the thread that runs the message loop: with OUTOFCONTEXT, Windows
/// delivers the callback through that thread's messages.
pub fn watch_foreground(on_change: fn()) {
    let _ = HANDLER.set(on_change);
    // SAFETY: `hook_proc` matches the signature Windows expects. The hook lives as long as
    // the process, so the handle is not kept.
    unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(hook_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
    }
}

unsafe extern "system" fn hook_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if let Some(handler) = HANDLER.get() {
        // Same reason as in window.rs: never unwind into Windows.
        let _ = std::panic::catch_unwind(handler);
    }
}

/// Does the window in front cover the whole of `monitor`? The desktop itself does (clicking
/// the wallpaper makes it the foreground window), so it is skipped on purpose.
pub fn foreground_covers(monitor: &Rect, own: HWND) -> bool {
    // SAFETY: all calls below only read window state; a window that vanishes mid-way just
    // makes a call fail, and we answer "no".
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() || fg == own || !IsWindowVisible(fg).as_bool() || IsIconic(fg).as_bool()
        {
            return false;
        }
        let mut class = [0u16; 64];
        let len = GetClassNameW(fg, &mut class) as usize;
        let class = String::from_utf16_lossy(&class[..len.min(class.len())]);
        if class == "Progman" || class == "WorkerW" || class == "Shell_TrayWnd" {
            return false;
        }
        let mut r = RECT::default();
        if GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        covers(&Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }, monitor)
    }
}
