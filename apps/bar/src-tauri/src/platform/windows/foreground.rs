//! Which program has the keyboard focus, and letting a client take it.
//!
//! A popover is the client's own window. Windows only lets a program take focus if it was just
//! allowed to: `allow(pid)` makes that grant right before `popover.show`. To know when the user
//! clicked somewhere else we do not poll: `watch` installs a hook that Windows calls when the
//! foreground window changes (rule 9: events, no polling).

use std::sync::OnceLock;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, GetForegroundWindow, GetWindowThreadProcessId,
    EVENT_SYSTEM_FOREGROUND, WINEVENT_OUTOFCONTEXT,
};

/// The process that has the foreground window right now, if there is one.
pub fn current_pid() -> Option<u32> {
    let mut pid = 0u32;
    // SAFETY: plain calls; the second only writes the pid we pass a pointer to.
    unsafe { GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid)) };
    (pid != 0).then_some(pid)
}

/// Lets process `pid` bring a window to the front. Only the bar's own process can grant this, and
/// the pid always comes from the pipe, never from what a client says about itself.
pub fn allow(pid: u32) {
    // SAFETY: plain call with a number. It fails if the process is gone, which is fine.
    if let Err(error) = unsafe { AllowSetForegroundWindow(pid) } {
        log::warn!("AllowSetForegroundWindow({pid}) failed: {error}");
    }
}

static HANDLER: OnceLock<fn(u32)> = OnceLock::new();

/// Windows calls this on the thread that installed the hook (it needs a message loop: the main
/// thread has one). `hwnd` is the window that just got the foreground.
unsafe extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    let mut pid = 0u32;
    // SAFETY: `hwnd` comes from Windows; the call only writes the pid we pass a pointer to.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    // pid 0 = no window got the foreground (a switch in progress): nothing to tell.
    if pid != 0 {
        if let Some(handler) = HANDLER.get() {
            handler(pid);
        }
    }
}

/// Calls `handler(pid)` whenever another program's window becomes the foreground. Call once, from
/// the main thread. The hook lives as long as the app, so its handle is never released.
pub fn watch(handler: fn(u32)) {
    if HANDLER.set(handler).is_err() {
        return;
    }
    // SAFETY: `on_foreground` has the signature Windows expects, and OUTOFCONTEXT means no DLL
    // is injected: Windows posts the event to our own message loop.
    let hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(on_foreground),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    if hook.is_invalid() {
        log::warn!("could not watch the foreground window: outside clicks will not close popovers");
    }
}

static WINDOW_HANDLER: OnceLock<fn(isize)> = OnceLock::new();

unsafe extern "system" fn on_foreground_window(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    if !hwnd.0.is_null() {
        if let Some(handler) = WINDOW_HANDLER.get() {
            handler(hwnd.0 as isize);
        }
    }
}

/// Like `watch`, but hands over the window handle (application menus need the window, not just the
/// process). A second hook: both can run. Call once, from the main thread.
pub fn watch_window(handler: fn(isize)) {
    if WINDOW_HANDLER.set(handler).is_err() {
        return;
    }
    // SAFETY: same as `watch`: OUTOFCONTEXT, no injection, events come to our own message loop.
    let hook = unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(on_foreground_window),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        )
    };
    if hook.is_invalid() {
        log::warn!("could not watch the foreground window: application menus will not follow the app in front");
    }
}

/// The window that has the foreground right now (0 if none).
pub fn current_window() -> isize {
    // SAFETY: plain call.
    unsafe { GetForegroundWindow().0 as isize }
}

/// Brings a window to the front. Allowed because the user just clicked the bar (our process has the
/// last input), which is how the app gets the focus back after one of its menu entries was picked.
pub fn focus(hwnd: isize) {
    use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
    // SAFETY: plain call; it just returns false if Windows refuses.
    unsafe {
        let _ = SetForegroundWindow(HWND(hwnd as *mut _));
    }
}
