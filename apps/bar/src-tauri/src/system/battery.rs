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
    }
}

pub fn service() -> &'static Service<Battery> {
    static S: OnceLock<Service<Battery>> = OnceLock::new();
    S.get_or_init(|| Service::new(Battery::default()))
}

/// What `set_saver` writes: the new levels and what to remember for later.
#[derive(Debug, PartialEq, Eq)]
pub struct SaverPlan {
    pub write: (u32, u32),
    pub restore: Option<[u32; 2]>,
}

/// Energy saver starts by itself at a charge level. 100 = on now, 0 = never. `current` is what the
/// power plan has, `restore` what we saved earlier, `percent` the charge (None = unknown) and
/// `on_battery` whether the PC runs on battery.
pub fn plan_saver(
    on: bool,
    current: (u32, u32),
    restore: Option<[u32; 2]>,
    percent: Option<u8>,
    on_battery: bool,
) -> SaverPlan {
    // Remember the user's own levels the first time, never our 100 or 0.
    let saved = restore.unwrap_or([current.0, current.1]);
    if on {
        return SaverPlan { write: (100, 100), restore: Some(saved) };
    }
    // Putting the levels back would switch it on again right away (battery already below the
    // user's level): write 0 instead and keep the saved levels for the next on/off.
    let would_restart = on_battery && percent.is_some_and(|p| u32::from(p) <= saved[1]);
    if would_restart {
        SaverPlan { write: (0, 0), restore: Some(saved) }
    } else {
        SaverPlan { write: (saved[0], saved[1]), restore: None }
    }
}

/// Turns energy saver on or off for real (no settings page).
pub fn set_saver(on: bool) {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    {
        use crate::platform::windows::power_scheme;
        let Some(current) = power_scheme::read() else { return };
        let Some(c) = crate::controller::get() else { return };
        let b = service().get();
        let plan = plan_saver(on, current, c.settings().saver_restore, b.percent, b.plugged_in == Some(false));
        if power_scheme::write(plan.write) {
            c.update_settings(|s| s.saver_restore = plan.restore);
        }
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
        service().set(from_status(
            p.ac_line,
            p.battery_flag,
            p.percent,
            p.seconds_left,
            saver,
        ));
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

    #[test]
    fn saver_on_remembers_the_users_levels_once() {
        let p = plan_saver(true, (0, 20), None, Some(80), true);
        assert_eq!(p, SaverPlan { write: (100, 100), restore: Some([0, 20]) });
        // Already switched on by us: the saved levels stay, not 100.
        let p = plan_saver(true, (100, 100), Some([0, 20]), Some(80), true);
        assert_eq!(p.restore, Some([0, 20]));
    }

    #[test]
    fn saver_off_puts_the_levels_back() {
        let p = plan_saver(false, (100, 100), Some([0, 20]), Some(80), true);
        assert_eq!(p, SaverPlan { write: (0, 20), restore: None });
    }

    #[test]
    fn saver_off_below_the_users_level_writes_never() {
        let p = plan_saver(false, (100, 100), Some([0, 20]), Some(15), true);
        assert_eq!(p, SaverPlan { write: (0, 0), restore: Some([0, 20]) });
        // Plugged in: nothing can restart it, so the levels go back.
        let p = plan_saver(false, (100, 100), Some([0, 20]), Some(15), false);
        assert_eq!(p.restore, None);
    }
}
