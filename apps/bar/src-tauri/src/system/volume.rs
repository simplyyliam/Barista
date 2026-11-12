//! Volume of the default output device: level, mute, and changes (also from the keyboard keys or
//! another app). Switching the output device is slice B.

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    /// False when there is no output device: the other fields mean nothing then.
    pub available: bool,
    /// 0.0 to 1.0.
    pub level: f64,
    pub muted: bool,
    pub device_name: String,
}

pub fn service() -> &'static Service<Volume> {
    static S: OnceLock<Service<Volume>> = OnceLock::new();
    S.get_or_init(|| Service::new(Volume::default()))
}

/// A level from the UI to what we send to Windows: NaN is refused, the rest is kept in 0 to 1.
pub fn clean_level(level: f64) -> Option<f32> {
    level.is_finite().then(|| level.clamp(0.0, 1.0) as f32)
}

/// Starts watching the output device.
pub fn start() {
    #[cfg(windows)]
    crate::platform::windows::audio::start(|raw| {
        service().set(match raw {
            Some(r) => Volume {
                available: true,
                level: f64::from(r.level),
                muted: r.muted,
                device_name: r.device_name,
            },
            None => Volume::default(),
        });
    });
}

pub fn set_volume(level: f64) {
    #[cfg(windows)]
    if let Some(level) = clean_level(level) {
        crate::platform::windows::audio::set_volume(level);
    }
    #[cfg(not(windows))]
    let _ = level;
}

pub fn set_muted(muted: bool) {
    #[cfg(windows)]
    crate::platform::windows::audio::set_muted(muted);
    #[cfg(not(windows))]
    let _ = muted;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_clamped_and_nan_is_refused() {
        assert_eq!(clean_level(0.5), Some(0.5));
        assert_eq!(clean_level(-3.0), Some(0.0));
        assert_eq!(clean_level(7.0), Some(1.0));
        assert_eq!(clean_level(f64::NAN), None);
        assert_eq!(clean_level(f64::INFINITY), None);
    }
}
