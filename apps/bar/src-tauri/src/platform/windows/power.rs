//! Power status and change notifications. `GetSystemPowerStatus` has the numbers; the
//! notifications only say "something changed", so every one makes us read the full status again.

use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, RegisterPowerSettingNotification, SYSTEM_POWER_STATUS,
};
use windows::Win32::System::SystemServices::{
    GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING, GUID_POWER_SAVING_STATUS,
};
use windows::Win32::UI::WindowsAndMessaging::DEVICE_NOTIFY_WINDOW_HANDLE;

/// The fields of `SYSTEM_POWER_STATUS`, unchanged.
pub struct RawPower {
    pub ac_line: u8,
    pub battery_flag: u8,
    pub percent: u8,
    pub seconds_left: u32,
    pub saver: u8,
}

pub fn read() -> Option<RawPower> {
    let mut s = SYSTEM_POWER_STATUS::default();
    // SAFETY: `s` is a plain struct that lives through the call.
    unsafe { GetSystemPowerStatus(&mut s) }.ok()?;
    Some(RawPower {
        ac_line: s.ACLineStatus,
        battery_flag: s.BatteryFlag,
        percent: s.BatteryLifePercent,
        seconds_left: s.BatteryLifeTime,
        saver: s.SystemStatusFlag,
    })
}

/// Asks Windows to send `WM_POWERBROADCAST` to `hwnd` when the charge, the power source or
/// battery saver changes. The registrations live as long as the window, so the handles are kept.
pub fn register(hwnd: HWND) {
    for guid in [
        GUID_BATTERY_PERCENTAGE_REMAINING,
        GUID_ACDC_POWER_SOURCE,
        GUID_POWER_SAVING_STATUS,
    ] {
        // SAFETY: the window is ours and outlives the registration.
        let result = unsafe {
            RegisterPowerSettingNotification(HANDLE(hwnd.0), &guid, DEVICE_NOTIFY_WINDOW_HANDLE)
        };
        if let Err(e) = result {
            eprintln!("barista: power notification not registered: {e}");
        }
    }
}
