//! Sends a keyboard shortcut to whatever window has the focus (ADR 0036, the fallback menus). This is
//! what the user would type, nothing more: `Ctrl+C` is `Ctrl+C` in every app.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_SHIFT,
};

fn key(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

/// The keys of a chord in the order they are pressed, then released in reverse.
pub fn chord(ctrl: bool, shift: bool, vk: u16) -> Vec<(u16, bool)> {
    let mut modifiers = Vec::new();
    if ctrl {
        modifiers.push(VK_CONTROL.0);
    }
    if shift {
        modifiers.push(VK_SHIFT.0);
    }
    let mut out: Vec<(u16, bool)> = modifiers.iter().map(|m| (*m, false)).collect();
    out.push((vk, false));
    out.push((vk, true));
    out.extend(modifiers.iter().rev().map(|m| (*m, true)));
    out
}

/// Types the chord (for example Ctrl+C) into the focused window. `false` if Windows refused part of it
/// (an elevated app in front: input to it is blocked).
pub fn send_chord(ctrl: bool, shift: bool, vk: u16) -> bool {
    let inputs: Vec<INPUT> = chord(ctrl, shift, vk).into_iter().map(|(k, up)| key(k, up)).collect();
    // SAFETY: the slice is ours and the size passed is the size of one `INPUT`.
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    sent as usize == inputs.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chord_presses_in_order_and_releases_in_reverse() {
        let keys = chord(true, true, 0x5A);
        assert_eq!(
            keys,
            vec![(VK_CONTROL.0, false), (VK_SHIFT.0, false), (0x5A, false), (0x5A, true), (VK_SHIFT.0, true), (VK_CONTROL.0, true)]
        );
        assert_eq!(chord(false, false, 0x41), vec![(0x41, false), (0x41, true)]);
    }
}
