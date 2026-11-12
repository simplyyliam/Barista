//! Battery: percent, charging, time left, battery saver. A desktop PC has no battery and reports
//! `present: false`. Changes come from `RegisterPowerSettingNotification` (see `window.rs`).

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    /// False on a desktop PC (no battery): the other fields mean nothing then.
    pub present: bool,
    /// 0 to 100, or `None` if Windows does not know.
    pub percent: Option<u8>,
    pub charging: bool,
    /// On mains power. `None` if unknown.
    pub plugged_in: Option<bool>,
    /// Seconds of battery left, only while unplugged and when Windows can estimate it.
    pub seconds_left: Option<u32>,
    /// Battery saver is on.
    pub saver: bool,
    /// The bar can switch battery saver itself. False until a call that Windows really acts on is
    /// found (ADR 0020); the tile then only shows the state.
    pub saver_controllable: bool,
}

/// `SYSTEM_POWER_STATUS` numbers to a snapshot. Windows uses 255 and `u32::MAX` for "unknown",
/// flag 128 for "no battery" and 8 for "charging".
pub fn from_status(
    ac_line: u8,
    battery_flag: u8,
    percent: u8,
    seconds_left: u32,
    saver: u8,
) -> Battery {
    if battery_flag == 255 || battery_flag & 128 != 0 {
        return Battery {
            plugged_in: (ac_line != 255).then_some(ac_line == 1),
            ..Battery::default()
        };
    }
    Battery {
        present: true,
        percent: (percent <= 100).then_some(percent),
        charging: battery_flag & 8 != 0,
        plugged_in: (ac_line != 255).then_some(ac_line == 1),
        seconds_left: (seconds_left != u32::MAX).then_some(seconds_left),
        saver: saver & 1 != 0,
        saver_controllable: false,
    }
}

pub fn service() -> &'static Service<Battery> {
    static S: OnceLock<Service<Battery>> = OnceLock::new();
    S.get_or_init(|| Service::new(Battery::default()))
}

/// Switches energy saver through the helper service. Nothing happens without it.
pub fn set_saver(on: bool) {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    {
        if !crate::platform::windows::saver_helper::set_saver(on) {
            eprintln!("barista: energy saver not switched (is the helper installed? `pnpm helper:install`)");
        }
        // The registry watch publishes the result; this covers a helper that changed nothing.
        refresh();
    }
    #[cfg(not(windows))]
    let _ = on;
}

/// Windows keeps the energy saver switch (Quick Settings) in this value: 1 = on, 2 = off. The power
/// status flag does not show it while plugged in, so we read the value too. Only Windows' own
/// service may write it (an ordinary program can't, it needs administrator rights).
#[cfg(windows)]
const SAVER_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Power";

/// Watches that value, so the tile follows a change made in Windows.
pub fn start_saver_watch() {
    #[cfg(windows)]
    if !crate::platform::windows::registry_watch::watch(SAVER_KEY, refresh) {
        eprintln!("barista: energy saver switch not watched");
    }
}

/// Reads the status from Windows and publishes it if it changed.
pub fn refresh() {
    #[cfg(windows)]
    if let Some(p) = crate::platform::windows::power::read() {
        let switch = crate::platform::windows::registry_watch::read_machine_dword(SAVER_KEY, "EnergySaverState");
        let saver = p.saver | u8::from(switch == Some(1));
        let mut battery = from_status(
            p.ac_line,
            p.battery_flag,
            p.percent,
            p.seconds_left,
            saver,
        );
        // Always false for now: the helper can write EnergySaverState, but Windows does not act on it
        // (tested 2026-11-12), so there is nothing the bar can call to switch energy saver (ADR 0020).
        battery.saver_controllable = false;
        service().set(battery);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_has_no_battery() {
        let b = from_status(1, 128, 255, u32::MAX, 0);
        assert!(!b.present);
        assert_eq!(b.percent, None);
        assert_eq!(b.plugged_in, Some(true));
    }

    #[test]
    fn charging_laptop() {
        let b = from_status(1, 8, 64, u32::MAX, 0);
        assert!(b.present && b.charging);
        assert_eq!(b.percent, Some(64));
        assert_eq!(b.seconds_left, None);
    }

    #[test]
    fn unplugged_with_estimate_and_saver() {
        let b = from_status(0, 1, 40, 7200, 1);
        assert!(b.present && !b.charging && b.saver);
        assert_eq!(b.plugged_in, Some(false));
        assert_eq!(b.seconds_left, Some(7200));
    }

    #[test]
    fn unknown_values_become_none() {
        let b = from_status(255, 2, 255, u32::MAX, 0);
        assert!(b.present);
        assert_eq!((b.percent, b.plugged_in), (None, None));
        // Battery flag 255 = unknown status: treated like no battery.
        assert!(!from_status(255, 255, 255, u32::MAX, 0).present);
    }
}
