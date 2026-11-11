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
    AllowSetForegroundWindow, GetWindowThreadProcessId, EVENT_SYSTEM_FOREGROUND,
    WINEVENT_OUTOFCONTEXT,
};

/// Lets process `pid` bring a window to the front. Only the bar's own process can grant this, and
/// the pid always comes from the pipe, never from what a client says about itself.
pub fn allow(pid: u32) {
    // SAFETY: plain call with a number. It fails if the process is gone, which is fine.
    if let Err(error) = unsafe { AllowSetForegroundWindow(pid) } {
        eprintln!("AllowSetForegroundWindow({pid}) failed: {error}");
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
        eprintln!("could not watch the foreground window: outside clicks will not close popovers");
    }
}
