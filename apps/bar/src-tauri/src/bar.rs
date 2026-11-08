//! The bar's plain-Rust logic: types, monitor choice and rectangle math. No Win32 in here, so
//! it is easy to unit test. `platform/windows/` does the calls, this decides what to ask for.

use serde::{Deserialize, Serialize};
use specta::Type;

/// A rectangle in **physical pixels** (what Windows works in). Logical pixels only exist in
/// settings and in `BarInfo.height`; `bar_rect` converts once (CLAUDE.md rule 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    /// Stable across restarts and replugging (the display's device path).
    pub id: String,
    pub name: String,
    pub is_primary: bool,
    /// Whole monitor.
    pub rect: Rect,
    /// Monitor minus the taskbar and other appbars (including ours).
    pub work_area: Rect,
    /// DPI scale: 1.0 = 100%, 1.25 = 125%, ...
    pub scale: f64,
}

/// What the frontend gets from `getBarInfo` and the `bar-changed` event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BarInfo {
    /// Bar height in logical pixels (the number to use in CSS).
    pub height: u32,
    /// Bar height in physical pixels on the current monitor.
    pub height_px: i32,
    pub scale: f64,
    pub monitor: Monitor,
    pub monitors: Vec<Monitor>,
    pub hidden_for_fullscreen: bool,
}

/// The saved monitor if it is plugged in, otherwise the primary one (or the first).
/// The saved id is never overwritten here: when the monitor comes back, the bar goes back.
pub fn pick_monitor<'a>(saved: Option<&str>, monitors: &'a [Monitor]) -> Option<&'a Monitor> {
    saved
        .and_then(|id| monitors.iter().find(|m| m.id == id))
        .or_else(|| monitors.iter().find(|m| m.is_primary))
        .or_else(|| monitors.first())
}

/// The strip the bar wants: the top of the monitor, full width. The logical height becomes
/// physical pixels here and nowhere else.
pub fn bar_rect(monitor: &Rect, scale: f64, height_logical: u32) -> Rect {
    let height_px = (f64::from(height_logical) * scale).round() as i32;
    Rect {
        left: monitor.left,
        top: monitor.top,
        right: monitor.right,
        bottom: monitor.top + height_px,
    }
}

/// A window is "full screen" when it covers the whole monitor, not just the work area.
/// A maximized window stops under the bar, so it does not count.
pub fn covers(window: &Rect, monitor: &Rect) -> bool {
    window.left <= monitor.left
        && window.top <= monitor.top
        && window.right >= monitor.right
        && window.bottom >= monitor.bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(l: i32, t: i32, r: i32, b: i32) -> Rect {
        Rect { left: l, top: t, right: r, bottom: b }
    }

    fn monitor(id: &str, primary: bool) -> Monitor {
        Monitor {
            id: id.into(),
            name: id.into(),
            is_primary: primary,
            rect: rect(0, 0, 1920, 1080),
            work_area: rect(0, 0, 1920, 1040),
            scale: 1.0,
        }
    }

    #[test]
    fn bar_height_scales_with_dpi() {
        let m = rect(0, 0, 1920, 1080);
        assert_eq!(bar_rect(&m, 1.0, 32).height(), 32);
        assert_eq!(bar_rect(&m, 1.25, 32).height(), 40);
        assert_eq!(bar_rect(&m, 1.5, 32).height(), 48);
    }

    #[test]
    fn bar_sits_at_the_top_of_a_monitor_with_an_offset() {
        // second monitor to the left of the primary: negative coordinates
        let m = rect(-1920, -200, 0, 880);
        let r = bar_rect(&m, 1.0, 32);
        assert_eq!((r.left, r.top, r.right, r.bottom), (-1920, -200, 0, -168));
    }

    #[test]
    fn saved_monitor_wins_then_primary_then_first() {
        let list = vec![monitor("a", true), monitor("b", false)];
        assert_eq!(pick_monitor(Some("b"), &list).unwrap().id, "b");
        assert_eq!(pick_monitor(Some("unplugged"), &list).unwrap().id, "a");
        assert_eq!(pick_monitor(None, &list).unwrap().id, "a");
        let no_primary = vec![monitor("x", false)];
        assert_eq!(pick_monitor(None, &no_primary).unwrap().id, "x");
        assert!(pick_monitor(None, &[]).is_none());
    }

    #[test]
    fn fullscreen_means_covering_the_whole_monitor() {
        let m = rect(0, 0, 1920, 1080);
        assert!(covers(&rect(0, 0, 1920, 1080), &m));
        assert!(covers(&rect(-8, -8, 1928, 1088), &m));
        assert!(!covers(&rect(0, 32, 1920, 1040), &m)); // maximized under the bar
    }
}
