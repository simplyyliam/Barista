//! Window corner radius: the plain-Rust rules, no Win32 in here so every rule is unit tested.
//! `platform/windows/windhawk.rs` reads the Windhawk settings, `platform/windows/window.rs`
//! tells us the window's state, and this file decides what radius that means.
//!
//! Windows 11 rounds the corners of a normal window by 8 px and squares them (0) when the
//! window is maximized, snapped or full screen. The Windhawk mod "custom-corner-radius" (it runs
//! inside `dwm.exe`) replaces the 8 with a radius of the user's choice and can optionally keep
//! the rounding for snapped and maximized windows. We copy that behaviour so the bar's corners
//! match what Windows draws for every other window.

use crate::bar::Rect;
use serde::{Deserialize, Serialize};
use specta::Type;

/// The radius of a normal window on stock Windows 11, in logical pixels (CSS px).
pub const DEFAULT_RADIUS: u32 = 8;

/// The mod's `roundMaximizedAndSnapped` setting: which squared-off windows keep their rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundMaximizedAndSnapped {
    /// `"none"` (and anything we don't know): snapped and maximized windows stay square.
    Never,
    /// `"snapped"`: snapped windows are rounded, maximized ones are square.
    Snapped,
    /// `"snappedAndMaximized"`: both are rounded.
    SnappedAndMaximized,
}

impl RoundMaximizedAndSnapped {
    pub fn parse(value: &str) -> Self {
        match value {
            "snapped" => Self::Snapped,
            "snappedAndMaximized" => Self::SnappedAndMaximized,
            _ => Self::Never,
        }
    }
}

/// The mod's settings, only when it is active. "Not active" is `None` everywhere else, so
/// there is no way to forget to check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModSettings {
    /// Corner radius in logical pixels.
    pub radius: u32,
    pub round_maximized_and_snapped: RoundMaximizedAndSnapped,
}

/// Turns the raw registry values into settings. `None` means the mod is not active:
///
/// - `disabled` is the DWORD `Disabled` of the mod's key: anything but 0 means switched off
///   (a missing value counts as 0, i.e. on).
/// - `radius` is the DWORD `radius` of its `Settings` key. Without it there is nothing to apply.
///   The mod stores "keep original" as -1, which is `0xFFFF_FFFF` as a DWORD, so the number is
///   read as signed and every negative value means "don't change anything".
/// - `round` is the string `roundMaximizedAndSnapped`.
pub fn settings_from_values(
    disabled: Option<u32>,
    radius: Option<u32>,
    round: Option<&str>,
) -> Option<ModSettings> {
    if disabled.unwrap_or(0) != 0 {
        return None;
    }
    let signed = radius? as i32;
    if signed < 0 {
        return None;
    }
    Some(ModSettings {
        radius: signed as u32,
        round_maximized_and_snapped: round.map_or(
            RoundMaximizedAndSnapped::Never,
            RoundMaximizedAndSnapped::parse,
        ),
    })
}

/// What a window is doing, as far as corners are concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    /// Docked to a half or quarter of the screen (Win+Arrow, drag to an edge, snap layouts).
    Snapped,
    Maximized,
    Fullscreen,
}

/// What the frontend gets from `getWindowRadius` and the `window-radius-changed` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowRadius {
    /// The OS window's corner radius in logical pixels, for working out child radii (window
    /// radius - padding). Not meant to be applied to the window itself. `None` only before
    /// the window's state could be read: the UI should then use its own default.
    pub radius: Option<u32>,
    /// True when the radius is not what stock Windows 11 would use for this window state, i.e.
    /// Windhawk is changing it.
    pub customized: bool,
}

/// What stock Windows 11 does: round a normal window, square everything else.
pub fn native_radius(state: WindowState) -> u32 {
    match state {
        WindowState::Normal => DEFAULT_RADIUS,
        WindowState::Snapped | WindowState::Maximized | WindowState::Fullscreen => 0,
    }
}

/// The radius a window of this state really has. `settings` is `None` when Windhawk's mod is
/// not active, which gives the stock Windows 11 behaviour.
pub fn effective_radius(state: WindowState, settings: Option<&ModSettings>) -> u32 {
    let Some(mod_settings) = settings else {
        return native_radius(state);
    };
    use RoundMaximizedAndSnapped as Round;
    match (state, mod_settings.round_maximized_and_snapped) {
        // A normal window simply gets the custom radius instead of the 8.
        (WindowState::Normal, _) => mod_settings.radius,
        // Full screen is always square, whatever the mod says.
        (WindowState::Fullscreen, _) => 0,
        // Snapped keeps the rounding for "snapped" and "snappedAndMaximized".
        (WindowState::Snapped, Round::Snapped | Round::SnappedAndMaximized) => mod_settings.radius,
        // Maximized keeps it only when the mod explicitly says so.
        (WindowState::Maximized, Round::SnappedAndMaximized) => mod_settings.radius,
        _ => 0,
    }
}

/// The value the UI gets for a window in `state`.
pub fn radius_for(state: WindowState, settings: Option<&ModSettings>) -> WindowRadius {
    let radius = effective_radius(state, settings);
    WindowRadius {
        radius: Some(radius),
        customized: radius != native_radius(state),
    }
}

/// Remembers the last value the UI heard, so an update is only sent when it really changed.
#[derive(Default)]
pub struct RadiusTracker {
    last: Option<WindowRadius>,
}

impl RadiusTracker {
    /// `Some(value)` when `next` differs from the last one (send it), `None` when it is the same.
    pub fn update(&mut self, next: WindowRadius) -> Option<WindowRadius> {
        if self.last == Some(next) {
            return None;
        }
        self.last = Some(next);
        Some(next)
    }

    /// The last value, or "unknown" before the first one.
    pub fn current(&self) -> WindowRadius {
        self.last.unwrap_or(WindowRadius {
            radius: None,
            customized: false,
        })
    }
}

/// Is a window docked to a snap position? Windows has no "is snapped" call, so we look at where
/// the window is: snapped windows sit flush against the edges of the work area (the screen
/// minus the taskbar and appbars) in a pattern that a normal window almost never has.
///
/// - `frame`: the visible window bounds (`DWMWA_EXTENDED_FRAME_BOUNDS`, without the invisible
///   resize border and shadow), physical pixels.
/// - `work`: the monitor's work area, physical pixels.
///
/// Snapped means one of: a left or right column (full height, at least a quarter of the width,
/// not the whole width), a top or bottom row (full width, at least a quarter of the height), or
/// a quarter in a corner (about half the width and half the height). Covering the whole work
/// area is a maximized window, not a snapped one.
pub fn snapped_by_geometry(frame: &Rect, work: &Rect) -> bool {
    // A few pixels of slack: DPI rounding, and Windows leaves a 1 px gap on some builds.
    const TOLERANCE: i32 = 8;
    let near = |a: i32, b: i32| (a - b).abs() <= TOLERANCE;

    let (width, height) = (frame.width(), frame.height());
    let (work_width, work_height) = (work.width(), work.height());
    if width <= 0 || height <= 0 || work_width <= 0 || work_height <= 0 {
        return false;
    }
    let left = near(frame.left, work.left);
    let right = near(frame.right, work.right);
    let top = near(frame.top, work.top);
    let bottom = near(frame.bottom, work.bottom);

    let full_width = left && right;
    let full_height = top && bottom;
    if full_width && full_height {
        return false;
    }
    if full_height && (left || right) {
        return width * 4 >= work_width;
    }
    if full_width && (top || bottom) {
        return height * 4 >= work_height;
    }
    let corner = (left || right) && (top || bottom);
    corner && near(width * 2, work_width) && near(height * 2, work_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use RoundMaximizedAndSnapped as Round;

    fn rect(l: i32, t: i32, r: i32, b: i32) -> Rect {
        Rect {
            left: l,
            top: t,
            right: r,
            bottom: b,
        }
    }

    fn mod_with(radius: u32, round: Round) -> ModSettings {
        ModSettings {
            radius,
            round_maximized_and_snapped: round,
        }
    }

    const ALL_STATES: [WindowState; 4] = [
        WindowState::Normal,
        WindowState::Snapped,
        WindowState::Maximized,
        WindowState::Fullscreen,
    ];

    #[test]
    fn stock_windows_11_without_the_mod() {
        assert_eq!(effective_radius(WindowState::Normal, None), 8);
        assert_eq!(effective_radius(WindowState::Snapped, None), 0);
        assert_eq!(effective_radius(WindowState::Maximized, None), 0);
        assert_eq!(effective_radius(WindowState::Fullscreen, None), 0);
    }

    #[test]
    fn a_normal_window_gets_the_custom_radius_whatever_the_option() {
        for round in [Round::Never, Round::Snapped, Round::SnappedAndMaximized] {
            assert_eq!(
                effective_radius(WindowState::Normal, Some(&mod_with(26, round))),
                26
            );
        }
    }

    #[test]
    fn fullscreen_is_always_square() {
        for round in [Round::Never, Round::Snapped, Round::SnappedAndMaximized] {
            assert_eq!(
                effective_radius(WindowState::Fullscreen, Some(&mod_with(26, round))),
                0
            );
        }
    }

    #[test]
    fn snapped_and_maximized_follow_the_option() {
        let cases = [
            (Round::Never, 0, 0),
            (Round::Snapped, 26, 0),
            (Round::SnappedAndMaximized, 26, 26),
        ];
        for (round, snapped, maximized) in cases {
            let settings = mod_with(26, round);
            assert_eq!(
                effective_radius(WindowState::Snapped, Some(&settings)),
                snapped,
                "{round:?}"
            );
            assert_eq!(
                effective_radius(WindowState::Maximized, Some(&settings)),
                maximized,
                "{round:?}"
            );
        }
    }

    #[test]
    fn a_custom_radius_of_zero_squares_a_normal_window() {
        assert_eq!(
            effective_radius(WindowState::Normal, Some(&mod_with(0, Round::Never))),
            0
        );
    }

    #[test]
    fn customized_means_different_from_stock_windows() {
        // Without the mod nothing is customized, in any state.
        for state in ALL_STATES {
            assert!(!radius_for(state, None).customized, "{state:?}");
        }
        // Radius 26 on a normal window: customized. On a square-by-default maximized window
        // with the option off: the result is the stock 0, so not customized.
        assert_eq!(
            radius_for(WindowState::Normal, Some(&mod_with(26, Round::Never))),
            WindowRadius {
                radius: Some(26),
                customized: true
            }
        );
        assert!(!radius_for(WindowState::Maximized, Some(&mod_with(26, Round::Never))).customized);
        assert!(
            radius_for(
                WindowState::Maximized,
                Some(&mod_with(26, Round::SnappedAndMaximized))
            )
            .customized
        );
        // A mod radius equal to the stock 8 changes nothing.
        assert!(!radius_for(WindowState::Normal, Some(&mod_with(8, Round::Never))).customized);
    }

    #[test]
    fn option_strings() {
        assert_eq!(Round::parse("snapped"), Round::Snapped);
        assert_eq!(
            Round::parse("snappedAndMaximized"),
            Round::SnappedAndMaximized
        );
        for other in ["none", "", "Snapped", "maximized", "snapped "] {
            assert_eq!(Round::parse(other), Round::Never, "{other:?}");
        }
    }

    #[test]
    fn raw_values_become_settings_or_none() {
        let active = settings_from_values(Some(0), Some(26), Some("snapped"));
        assert_eq!(active, Some(mod_with(26, Round::Snapped)));
        // No Disabled value = enabled; no option string = never.
        assert_eq!(
            settings_from_values(None, Some(5), None),
            Some(mod_with(5, Round::Never))
        );
        // 0 is a real radius (square corners), not "off".
        assert_eq!(
            settings_from_values(Some(0), Some(0), None),
            Some(mod_with(0, Round::Never))
        );
    }

    #[test]
    fn disabled_missing_and_keep_original_mean_not_active() {
        assert_eq!(
            settings_from_values(Some(1), Some(26), None),
            None,
            "Disabled = 1"
        );
        assert_eq!(
            settings_from_values(Some(7), Some(26), None),
            None,
            "Disabled != 0"
        );
        assert_eq!(
            settings_from_values(Some(0), None, None),
            None,
            "no radius value"
        );
        assert_eq!(
            settings_from_values(Some(0), Some(u32::MAX), None),
            None,
            "-1 = keep original"
        );
        assert_eq!(
            settings_from_values(Some(0), Some((-5i32) as u32), None),
            None,
            "negative"
        );
    }

    #[test]
    fn tracker_reports_only_changes() {
        let mut tracker = RadiusTracker::default();
        assert_eq!(
            tracker.current(),
            WindowRadius {
                radius: None,
                customized: false
            }
        );
        let eight = radius_for(WindowState::Normal, None);
        assert_eq!(
            tracker.update(eight),
            Some(eight),
            "first value is always news"
        );
        assert_eq!(tracker.update(eight), None);
        assert_eq!(tracker.update(eight), None);
        let square = radius_for(WindowState::Maximized, None);
        assert_eq!(tracker.update(square), Some(square));
        assert_eq!(tracker.current(), square);
        assert_eq!(tracker.update(eight), Some(eight), "and back again");
    }

    // A 1920x1040 work area (1080 screen minus a 40 px taskbar).
    const WORK: Rect = Rect {
        left: 0,
        top: 0,
        right: 1920,
        bottom: 1040,
    };

    #[test]
    fn snapped_halves_and_quarters() {
        assert!(
            snapped_by_geometry(&rect(0, 0, 960, 1040), &WORK),
            "left half"
        );
        assert!(
            snapped_by_geometry(&rect(960, 0, 1920, 1040), &WORK),
            "right half"
        );
        assert!(
            snapped_by_geometry(&rect(0, 0, 1280, 1040), &WORK),
            "two thirds"
        );
        assert!(
            snapped_by_geometry(&rect(0, 0, 960, 520), &WORK),
            "top left quarter"
        );
        assert!(
            snapped_by_geometry(&rect(960, 520, 1920, 1040), &WORK),
            "bottom right quarter"
        );
        assert!(
            snapped_by_geometry(&rect(0, 0, 1920, 520), &WORK),
            "top half"
        );
        assert!(
            snapped_by_geometry(&rect(1, 1, 959, 1039), &WORK),
            "tolerance for rounding"
        );
    }

    #[test]
    fn not_snapped() {
        assert!(
            !snapped_by_geometry(&rect(0, 0, 1920, 1040), &WORK),
            "maximized"
        );
        assert!(
            !snapped_by_geometry(&rect(200, 100, 1200, 800), &WORK),
            "floating"
        );
        assert!(
            !snapped_by_geometry(&rect(0, 100, 960, 800), &WORK),
            "touches only one edge"
        );
        assert!(
            !snapped_by_geometry(&rect(0, 0, 400, 1040), &WORK),
            "thin strip, under a quarter"
        );
        assert!(
            !snapped_by_geometry(&rect(0, 0, 1920, 48), &WORK),
            "a thin bar across the top"
        );
        assert!(
            !snapped_by_geometry(&rect(0, 0, 500, 300), &WORK),
            "small window in a corner"
        );
        assert!(!snapped_by_geometry(&rect(0, 0, 0, 0), &WORK), "empty");
    }

    #[test]
    fn snapping_on_a_monitor_that_does_not_start_at_zero() {
        let work = rect(-1920, 200, 0, 1240);
        assert!(snapped_by_geometry(&rect(-1920, 200, -960, 1240), &work));
        assert!(!snapped_by_geometry(&rect(-1800, 300, -1000, 900), &work));
    }
}
