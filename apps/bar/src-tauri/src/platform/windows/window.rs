//! The bar window's Win32 side: styles, placement, show/hide, and a hook that turns the
//! window messages we care about into plain `WindowEvent`s for the rest of the app.

use super::appbar;
use crate::bar::Rect;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, RegisterWindowMessageW, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE, WM_ACTIVATE,
    WM_DISPLAYCHANGE, WM_DPICHANGED, WM_WINDOWPOSCHANGED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

/// What happened to the screen or the shell, in our own words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    /// Another appbar (or the taskbar) moved; our strip may need to move too.
    PosChanged,
    /// Windows says a full-screen app started (`true`) or ended (`false`).
    FullscreenApp(bool),
    DisplayChanged,
    DpiChanged,
    /// Explorer restarted and forgot our appbar.
    TaskbarCreated,
}

// ABN_* notification codes, sent in WPARAM with `appbar::CALLBACK_MSG`.
const ABN_POSCHANGED: usize = 1;
const ABN_FULLSCREENAPP: usize = 2;

static HANDLER: OnceLock<fn(WindowEvent)> = OnceLock::new();
/// Id of the "TaskbarCreated" message Explorer broadcasts after it restarts.
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

fn hwnd_from(raw: isize) -> HWND {
    HWND(raw as *mut _)
}

/// Makes the window never take focus and keeps it out of Alt+Tab and the taskbar.
/// Call while the window is still hidden.
pub fn set_no_focus_styles(hwnd: HWND) {
    // SAFETY: plain get/set of the window's extended style bits.
    unsafe {
        let old = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let add = (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, old | add);
    }
}

/// Moves and sizes the window (physical pixels) above normal windows, without activating it.
pub fn place(hwnd: HWND, r: Rect) {
    // SAFETY: plain call; failure (window gone during exit) is harmless, so it is ignored.
    let _ = unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            r.left,
            r.top,
            r.width(),
            r.height(),
            SWP_NOACTIVATE,
        )
    };
}

pub fn set_visible(hwnd: HWND, visible: bool) {
    // SAFETY: plain call. SW_SHOWNOACTIVATE shows without stealing focus.
    unsafe {
        let _ = ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
    }
}

/// Starts listening to the window's messages. `handler` runs on the UI thread.
pub fn install(raw_hwnd: isize, handler: fn(WindowEvent)) {
    let _ = HANDLER.set(handler);
    // SAFETY: registering a message name is always allowed; the id is process-wide.
    let id = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    TASKBAR_CREATED.store(id, Ordering::SeqCst);
    // SAFETY: the subclass proc below has the exact signature Windows expects. Subclassing
    // puts us in front of Tauri's own handler: we see each message first and pass it on.
    unsafe {
        let _ = SetWindowSubclass(hwnd_from(raw_hwnd), Some(subclass_proc), 0xBA_0001, 0);
    }
}

fn emit(event: WindowEvent) {
    if let Some(handler) = HANDLER.get() {
        // A panic must not unwind through Windows' code (that aborts the process), so catch
        // it. The panic hook has already given the screen space back by then.
        let _ = std::panic::catch_unwind(|| handler(event));
    }
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    // Tauri handles the message first. This matters for WM_DPICHANGED: Tauri resizes the
    // window to Windows' suggestion, and only then do we put it back to our own rectangle.
    // SAFETY: forwarding the same arguments we were given.
    let result = unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };

    match msg {
        m if m == appbar::CALLBACK_MSG => match wparam.0 {
            ABN_POSCHANGED => emit(WindowEvent::PosChanged),
            ABN_FULLSCREENAPP => emit(WindowEvent::FullscreenApp(lparam.0 != 0)),
            _ => {}
        },
        WM_DISPLAYCHANGE => emit(WindowEvent::DisplayChanged),
        WM_DPICHANGED => emit(WindowEvent::DpiChanged),
        WM_ACTIVATE => appbar::activate(hwnd),
        WM_WINDOWPOSCHANGED => appbar::window_pos_changed(hwnd),
        m if m != 0 && m == TASKBAR_CREATED.load(Ordering::SeqCst) => {
            emit(WindowEvent::TaskbarCreated)
        }
        _ => {}
    }
    result
}
