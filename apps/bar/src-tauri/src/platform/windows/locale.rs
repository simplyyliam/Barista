//! Regional settings the clock needs: the time format, the first day of the week and the time
//! zone. Windows only gives raw strings and numbers; turning them into values is in `system/clock.rs`.

use windows::core::PCWSTR;
use windows::Win32::Globalization::{GetLocaleInfoEx, LOCALE_IFIRSTDAYOFWEEK, LOCALE_STIMEFORMAT};
use windows::Win32::System::Time::{GetDynamicTimeZoneInformation, DYNAMIC_TIME_ZONE_INFORMATION};

/// What Windows said, before interpretation.
pub struct RawClock {
    /// Long time pattern, e.g. `HH:mm:ss` or `h:mm:ss tt`.
    pub time_format: String,
    /// `LOCALE_IFIRSTDAYOFWEEK`: 0 = Monday ... 6 = Sunday.
    pub first_day: u32,
    /// Windows time zone id, e.g. `W. Europe Standard Time`.
    pub zone_id: String,
    /// Minutes to add to UTC for local time, daylight saving included.
    pub utc_offset_minutes: i32,
}

fn locale_string(kind: u32) -> String {
    let mut buf = [0u16; 80];
    // SAFETY: a null name means "the user's locale"; the buffer outlives the call.
    let n = unsafe { GetLocaleInfoEx(PCWSTR::null(), kind, Some(&mut buf)) };
    String::from_utf16_lossy(&buf[..(n.max(1) as usize - 1).min(buf.len())])
}

pub fn read() -> RawClock {
    let first_day = locale_string(LOCALE_IFIRSTDAYOFWEEK).trim().parse().unwrap_or(0);
    let mut tz = DYNAMIC_TIME_ZONE_INFORMATION::default();
    // SAFETY: `tz` is a plain struct that lives through the call.
    let kind = unsafe { GetDynamicTimeZoneInformation(&mut tz) };
    // 1 = standard time, 2 = daylight time; anything else uses the plain bias.
    let extra = match kind {
        1 => tz.StandardBias,
        2 => tz.DaylightBias,
        _ => 0,
    };
    let len = tz.TimeZoneKeyName.iter().position(|&c| c == 0).unwrap_or(0);
    RawClock {
        time_format: locale_string(LOCALE_STIMEFORMAT),
        first_day,
        zone_id: String::from_utf16_lossy(&tz.TimeZoneKeyName[..len]),
        // Windows' bias is "UTC minus local", so local = -(bias + extra).
        utc_offset_minutes: -(tz.Bias + extra),
    }
}
