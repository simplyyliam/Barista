//! Brightness of the laptop screen (WMI) and of external monitors (DDC/CI). A display that does
//! not answer, or answers too slowly, is listed with `supported: false` so the UI can grey it out
//! (ADR 0019). Changes made with the brightness keys arrive live; an external monitor's own buttons
//! are not seen until the next read (the control center opening, monitors changing).

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

/// The lowest level we set: 0 can turn a screen fully black, and the user may not find the way back.
const MIN_PERCENT: u8 = 1;
const MAX_ID: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    /// Pass it back to `setBrightness` unchanged.
    pub id: String,
    pub name: String,
    /// The laptop's own panel.
    pub internal: bool,
    /// False: the display did not answer (no DDC/CI, a dock, too slow). `level` means nothing.
    pub supported: bool,
    /// 0.0 to 1.0.
    pub level: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Brightness {
    /// The laptop panel first, then external monitors. Empty before the first read.
    pub displays: Vec<Display>,
}

pub fn service() -> &'static Service<Brightness> {
    static S: OnceLock<Service<Brightness>> = OnceLock::new();
    S.get_or_init(|| Service::new(Brightness::default()))
}

/// A UI level (0..1) to the percent we send. NaN is refused; the result is never below 1.
pub fn to_percent(level: f64) -> Option<u8> {
    level
        .is_finite()
        .then(|| ((level.clamp(0.0, 1.0) * 100.0).round() as u8).max(MIN_PERCENT))
}

#[cfg(windows)]
fn changed(raw: Vec<crate::platform::windows::display_brightness::RawDisplay>) {
    if !super::control_center_active() {
        return;
    }
    super::publish_gated(service(), Brightness {
        displays: raw
            .into_iter()
            .map(|d| Display {
                supported: d.percent.is_some(),
                level: f64::from(d.percent.unwrap_or(0)) / 100.0,
                id: d.id,
                name: d.name,
                internal: d.internal,
            })
            .collect(),
    });
}

pub fn start() {
    #[cfg(windows)]
    crate::platform::windows::display_brightness::start(changed);
}

/// Reads every display again (monitors plugged in or out, the control center opened).
pub fn refresh() {
    #[cfg(windows)]
    crate::platform::windows::display_brightness::refresh_all();
}

/// Only ids we listed, and only displays that answered, reach the hardware.
pub fn set(id: &str, level: f64) {
    if !super::control_center_active() || id.len() > MAX_ID {
        return;
    }
    let Some(percent) = to_percent(level) else { return };
    let known = service()
        .get()
        .displays
        .iter()
        .any(|d| d.id == id && d.supported);
    if !known {
        return;
    }
    #[cfg(windows)]
    crate::platform::windows::display_brightness::set(id.to_owned(), percent);
    #[cfg(not(windows))]
    let _ = percent;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_become_percent_never_zero() {
        assert_eq!(to_percent(0.5), Some(50));
        assert_eq!(to_percent(1.0), Some(100));
        assert_eq!(to_percent(0.0), Some(1));
        assert_eq!(to_percent(-4.0), Some(1));
        assert_eq!(to_percent(9.0), Some(100));
        assert_eq!(to_percent(f64::NAN), None);
        assert_eq!(to_percent(f64::INFINITY), None);
    }
}
