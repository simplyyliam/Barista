//! Windows' own taskbar: read and set its auto-hide flag (ADR 0031). The taskbar is also an
//! appbar, so `ABM_SETSTATE` is the supported way (no hiding Explorer's window by force).
//! We remember the state from before we changed it and put it back on quit.

use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::Shell::{SHAppBarMessage, ABM_GETSTATE, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA};

fn data(hwnd: HWND, state: u32) -> APPBARDATA {
    APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        lParam: LPARAM(state as isize),
        ..Default::default()
    }
}

/// The taskbar's state bits (`ABS_AUTOHIDE` = 1, `ABS_ALWAYSONTOP` = 2).
pub fn state() -> u32 {
    let mut d = data(HWND::default(), 0);
    // SAFETY: `d` is a valid APPBARDATA with its size set.
    unsafe { SHAppBarMessage(ABM_GETSTATE, &mut d) as u32 }
}

/// Writes the state bits. `hwnd` is any of our windows (Windows wants one).
pub fn set_state(hwnd: HWND, state: u32) {
    let mut d = data(hwnd, state);
    // SAFETY: `d` is a valid APPBARDATA with its size set.
    unsafe {
        SHAppBarMessage(ABM_SETSTATE, &mut d);
    }
}

/// `state` with the auto-hide bit on or off, other bits untouched.
pub fn with_autohide(state: u32, on: bool) -> u32 {
    if on {
        state | ABS_AUTOHIDE
    } else {
        state & !ABS_AUTOHIDE
    }
}

#[cfg(test)]
pub fn is_autohide(state: u32) -> bool {
    state & ABS_AUTOHIDE != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_autohide_bit_changes() {
        assert_eq!(with_autohide(2, true), 3);
        assert_eq!(with_autohide(3, false), 2);
        assert!(is_autohide(3) && !is_autohide(2));
    }
}
