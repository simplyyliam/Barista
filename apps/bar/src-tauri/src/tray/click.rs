//! What a click on a tray icon sends to the app that owns it (ADR 0027). Pure Rust: the posting
//! itself is in `platform/windows/tray_input.rs`, so everything here is tested with plain values.
//!
//! Explorer posts a short run of mouse messages to the icon's callback message, not one. Apps
//! expect that run (some open on `WM_LBUTTONUP`, some on `NIN_SELECT`), so we send the same one.

use barista_protocol::MouseButton;

pub const WM_MOUSEMOVE: u32 = 0x0200;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_LBUTTONDBLCLK: u32 = 0x0203;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
pub const WM_MBUTTONDOWN: u32 = 0x0207;
pub const WM_MBUTTONUP: u32 = 0x0208;
pub const WM_CONTEXTMENU: u32 = 0x007B;
/// Version 4 only: "the user selected the icon".
pub const NIN_SELECT: u32 = 0x0400;

/// Icons that asked for `NOTIFYICON_VERSION_4` get the anchor in `wParam`, `WM_CONTEXTMENU` and
/// `NIN_SELECT`. Older ones only get the plain mouse messages.
pub const VERSION_4: u32 = 4;

/// The run of messages for one click. `double` is a left click that follows another within the
/// system's double-click time (right and middle have no double click for tray icons).
pub fn sequence(button: MouseButton, version: u32, double: bool) -> Vec<u32> {
    let v4 = version >= VERSION_4;
    match button {
        MouseButton::Left if double => {
            let mut run = vec![WM_LBUTTONDBLCLK, WM_LBUTTONUP];
            if v4 {
                run.push(NIN_SELECT);
            }
            run
        }
        MouseButton::Left => {
            let mut run = vec![WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP];
            if v4 {
                run.push(NIN_SELECT);
            }
            run
        }
        MouseButton::Right => {
            let mut run = vec![WM_MOUSEMOVE, WM_RBUTTONDOWN, WM_RBUTTONUP];
            if v4 {
                run.push(WM_CONTEXTMENU);
            }
            run
        }
        MouseButton::Middle => vec![WM_MBUTTONDOWN, WM_MBUTTONUP],
    }
}

/// `wParam` and `lParam` for one message. `(x, y)` is the anchor in screen pixels, `id` the icon's
/// `uID`. Version 4 packs them as `wParam = MAKEWPARAM(x, y)`, `lParam = MAKELPARAM(msg, id)`; the
/// older versions send `wParam = id`, `lParam = msg`.
pub fn encode(message: u32, version: u32, id: u32, (x, y): (i32, i32)) -> (usize, isize) {
    if version >= VERSION_4 {
        // Coordinates are signed 16-bit words (a monitor left of the primary one is negative).
        let w = (x as i16 as u16 as u32) | ((y as i16 as u16 as u32) << 16);
        let l = (message & 0xFFFF) | ((id & 0xFFFF) << 16);
        (w as usize, l as i32 as isize)
    } else {
        (id as usize, message as isize)
    }
}

/// Remembers the last left click, so a second one on the same icon within the double-click time
/// becomes a double click. Time comes in as a parameter so tests need no clock.
#[derive(Default)]
pub struct DoubleClick {
    last: Option<(String, u64)>,
}

impl DoubleClick {
    /// For a `static` (`Default` is not `const`).
    pub const fn default_const() -> Self {
        Self { last: None }
    }

    /// Call for every left click. True when this one is the second of a double click.
    pub fn left(&mut self, key: &str, now_ms: u64, window_ms: u64) -> bool {
        let double = matches!(&self.last, Some((k, t)) if k == key && now_ms.saturating_sub(*t) <= window_ms);
        // A double click uses up both clicks: the next one starts a new pair.
        self.last = if double { None } else { Some((key.to_string(), now_ms)) };
        double
    }

    /// Any other click (another button, another icon) breaks a pair in progress.
    pub fn reset(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_left_click_is_move_down_up() {
        assert_eq!(
            sequence(MouseButton::Left, 0, false),
            [WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP]
        );
    }

    #[test]
    fn version_4_adds_select_and_context_menu() {
        assert_eq!(sequence(MouseButton::Left, 4, false).last(), Some(&NIN_SELECT));
        assert_eq!(sequence(MouseButton::Right, 4, false).last(), Some(&WM_CONTEXTMENU));
        assert_eq!(sequence(MouseButton::Right, 0, false).last(), Some(&WM_RBUTTONUP));
    }

    #[test]
    fn double_click_and_middle() {
        assert_eq!(sequence(MouseButton::Left, 0, true), [WM_LBUTTONDBLCLK, WM_LBUTTONUP]);
        // Recorded from Explorer (docs/decisions/0027): a version 4 double click ends with NIN_SELECT.
        assert_eq!(
            sequence(MouseButton::Left, 4, true),
            [WM_LBUTTONDBLCLK, WM_LBUTTONUP, NIN_SELECT]
        );
        assert_eq!(sequence(MouseButton::Middle, 4, false), [WM_MBUTTONDOWN, WM_MBUTTONUP]);
    }

    #[test]
    fn old_versions_send_the_id_in_wparam() {
        assert_eq!(encode(WM_RBUTTONUP, 0, 7, (100, 50)), (7, WM_RBUTTONUP as isize));
    }

    #[test]
    fn version_4_packs_anchor_message_and_id() {
        let (w, l) = encode(WM_CONTEXTMENU, 4, 2, (100, 50));
        assert_eq!(w, 100 | (50 << 16));
        assert_eq!(l, (WM_CONTEXTMENU | (2 << 16)) as isize);
    }

    #[test]
    fn negative_coordinates_stay_in_their_word() {
        let (w, _) = encode(WM_LBUTTONUP, 4, 1, (-20, 5));
        assert_eq!(w & 0xFFFF, 0xFFEC); // -20 as i16
        assert_eq!(w >> 16, 5);
    }

    #[test]
    fn second_click_in_time_is_a_double_click() {
        let mut d = DoubleClick::default();
        assert!(!d.left("tray/a", 1000, 500));
        assert!(d.left("tray/a", 1300, 500));
        // The pair is used up: a third click starts over.
        assert!(!d.left("tray/a", 1400, 500));
    }

    #[test]
    fn slow_or_other_icon_or_other_button_is_not_a_double_click() {
        let mut d = DoubleClick::default();
        d.left("tray/a", 1000, 500);
        assert!(!d.left("tray/a", 1600, 500));
        assert!(!d.left("tray/b", 1700, 500));
        d.reset();
        assert!(!d.left("tray/b", 1800, 500));
    }
}
