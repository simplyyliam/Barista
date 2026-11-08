//! AppBar: how Windows lets a window reserve a strip of the screen (the taskbar uses it too).
//! Maximized windows then stop at the edge of that strip. We use it for the top edge.
//!
//! The flow: `register` once, `set_pos` whenever the bar's rectangle changes, `unregister`
//! on exit. If we never unregister, the strip stays reserved (see ADR 0002).

use crate::bar::Rect;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_TOP, ABM_ACTIVATE, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS,
    ABM_WINDOWPOSCHANGED, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

/// The message Windows sends our window about appbar events (position changed, a full-screen
/// app appeared). Any number above WM_APP is ours to pick.
pub const CALLBACK_MSG: u32 = WM_APP + 1;

/// The registered window, kept in a plain atomic so the panic hook can unregister without
/// taking any lock (a lock could be held by the thread that just panicked).
static REGISTERED: AtomicIsize = AtomicIsize::new(0);

fn data(hwnd: HWND) -> APPBARDATA {
    APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        uCallbackMessage: CALLBACK_MSG,
        uEdge: ABE_TOP,
        ..Default::default()
    }
}

fn to_win(r: Rect) -> RECT {
    RECT { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

fn from_win(r: RECT) -> Rect {
    Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

/// Tells Windows this window is an appbar. Returns false if Windows refused.
pub fn register(hwnd: HWND) -> bool {
    let mut d = data(hwnd);
    // SAFETY: `d` is a fully initialised APPBARDATA that lives through the call.
    let ok = unsafe { SHAppBarMessage(ABM_NEW, &mut d) } != 0;
    if ok {
        REGISTERED.store(hwnd.0 as isize, Ordering::SeqCst);
    }
    ok
}

/// Gives the reserved space back. Safe to call twice and from any thread.
pub fn unregister() {
    // swap(0) makes sure only one caller (exit, panic hook, ...) actually sends ABM_REMOVE.
    let raw = REGISTERED.swap(0, Ordering::SeqCst);
    if raw == 0 {
        return;
    }
    let mut d = data(HWND(raw as *mut _));
    // SAFETY: same as `register`.
    unsafe { SHAppBarMessage(ABM_REMOVE, &mut d) };
}

pub fn is_registered() -> bool {
    REGISTERED.load(Ordering::SeqCst) != 0
}

/// Forgets the registration without telling Windows (used when explorer restarted and
/// already lost it), so `register` can run again.
pub fn forget() {
    REGISTERED.store(0, Ordering::SeqCst);
}

/// Reserves `desired` (a strip at the top of a monitor) and returns the rectangle Windows
/// actually granted. Windows may push the strip down if another appbar already sits there.
pub fn set_pos(hwnd: HWND, desired: Rect) -> Rect {
    let height = desired.height();
    let mut d = data(hwnd);
    d.rc = to_win(desired);
    // SAFETY: as in `register`. QUERYPOS adjusts `d.rc` so it doesn't overlap other appbars...
    unsafe { SHAppBarMessage(ABM_QUERYPOS, &mut d) };
    // ...but for a top bar it only moves the top edge, so we restore our own height.
    d.rc.bottom = d.rc.top + height;
    // SAFETY: as above. SETPOS makes the reservation real; maximized windows shrink now.
    unsafe { SHAppBarMessage(ABM_SETPOS, &mut d) };
    from_win(d.rc)
}

/// Windows asks every appbar to forward these two window messages. Skipping them can
/// confuse the taskbar's z-order handling.
pub fn activate(hwnd: HWND) {
    let mut d = data(hwnd);
    // SAFETY: as in `register`.
    unsafe { SHAppBarMessage(ABM_ACTIVATE, &mut d) };
}

pub fn window_pos_changed(hwnd: HWND) {
    let mut d = data(hwnd);
    // SAFETY: as in `register`.
    unsafe { SHAppBarMessage(ABM_WINDOWPOSCHANGED, &mut d) };
}
