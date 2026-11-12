//! Clock settings: 12 or 24 hour, first day of the week, time zone. The time itself comes from
//! JS (`Date`); this only says how the user wants it shown. Refreshed on `WM_TIMECHANGE` and when
//! the regional settings change.

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClockSettings {
    /// True for a 24 hour clock.
    pub hour24: bool,
    /// 0 = Sunday ... 6 = Saturday (the same numbers as JS `Date.getDay()`).
    pub first_weekday: u8,
    /// Windows time zone id (`W. Europe Standard Time`). Empty if Windows did not say.
    pub time_zone: String,
    /// Minutes east of UTC right now, daylight saving included.
    pub utc_offset_minutes: i32,
}

impl Default for ClockSettings {
    fn default() -> Self {
        Self {
            hour24: true,
            first_weekday: 1,
            time_zone: String::new(),
            utc_offset_minutes: 0,
        }
    }
}

/// A time pattern is 24 hour when it has `H` (`HH:mm`), 12 hour when it has `h` (`h:mm tt`).
pub fn is_24h(pattern: &str) -> bool {
    // Text in quotes is literal ('h' in a language's word), so skip it.
    let mut quoted = false;
    for c in pattern.chars() {
        match c {
            '\'' => quoted = !quoted,
            'H' if !quoted => return true,
            'h' if !quoted => return false,
            _ => {}
        }
    }
    true
}

/// Windows counts 0 = Monday ... 6 = Sunday; JS counts 0 = Sunday.
pub fn weekday_from_windows(v: u32) -> u8 {
    ((v.min(6) + 1) % 7) as u8
}

pub fn service() -> &'static Service<ClockSettings> {
    static S: OnceLock<Service<ClockSettings>> = OnceLock::new();
    S.get_or_init(|| Service::new(ClockSettings::default()))
}

/// Reads the settings from Windows and publishes them if they changed.
pub fn refresh() {
    #[cfg(windows)]
    {
        let raw = crate::platform::windows::locale::read();
        service().set(ClockSettings {
            hour24: is_24h(&raw.time_format),
            first_weekday: weekday_from_windows(raw.first_day),
            time_zone: raw.zone_id,
            utc_offset_minutes: raw.utc_offset_minutes,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_the_hour_cycle() {
        assert!(is_24h("HH:mm:ss"));
        assert!(is_24h("H:mm"));
        assert!(!is_24h("h:mm:ss tt"));
        assert!(!is_24h("hh:mm:ss tt"));
        // A quoted letter is text, not a field.
        assert!(is_24h("'h' HH:mm"));
        assert!(is_24h(""));
    }

    #[test]
    fn weekday_numbers_match_js() {
        assert_eq!(weekday_from_windows(0), 1); // Monday
        assert_eq!(weekday_from_windows(6), 0); // Sunday
        assert_eq!(weekday_from_windows(5), 6); // Saturday
        assert_eq!(weekday_from_windows(99), 0); // junk is treated as Sunday
    }
}
