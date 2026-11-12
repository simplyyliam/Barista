//! Airplane mode switch. Windows has no public API for it; Quick Settings talks to the radio
//! management service through `IRadioManager`, an undocumented COM interface (ADR 0020). Same
//! approach as `audio_policy.rs`: only the slots we need are declared, in vtable order. If Windows
//! changes it, the call fails and the tile shows as unavailable (never a link to the settings page).

#![allow(non_snake_case)]

use core::ffi::c_void;
use windows::core::{interface, IUnknown, IUnknown_Vtbl, GUID, HRESULT};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

#[interface("db3afbfb-08e6-46c6-aa70-bf9a34c30ab7")]
unsafe trait IRadioManager: IUnknown {
    fn IsRMSupported(&self, supported: *mut u32) -> HRESULT;
    fn GetUIRadioInstances(&self, instances: *mut *mut c_void) -> HRESULT;
    fn GetSystemRadioState(&self, enabled: *mut i32, hardware_blocked: *mut i32, reason: *mut i32) -> HRESULT;
    fn SetSystemRadioState(&self, enabled: i32) -> HRESULT;
    fn Refresh(&self) -> HRESULT;
    fn OnHardwareSlideSwitchChanged(&self) -> HRESULT;
}

/// `RadioManagementAPI`.
const CLSID_RADIO_MANAGEMENT: GUID = GUID::from_u128(0x581333f6_28db_41be_bc7a_ff201f12f3f6);

fn manager() -> Option<IRadioManager> {
    // SAFETY: COM is started on this thread (a second start is harmless); the class id is Windows'.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        CoCreateInstance(&CLSID_RADIO_MANAGEMENT, None, CLSCTX_ALL).ok()
    }
}

/// True when all radios are on (airplane mode off). `None` if Windows doesn't answer.
pub fn radios_enabled() -> Option<bool> {
    let m = manager()?;
    let (mut enabled, mut blocked, mut reason) = (0i32, 0i32, 0i32);
    // SAFETY: the three out pointers live for the call.
    unsafe { m.GetSystemRadioState(&mut enabled, &mut blocked, &mut reason) }.ok().ok()?;
    Some(enabled != 0)
}

/// Switches airplane mode. Returns false if Windows refused or the interface is gone.
pub fn set_airplane(on: bool) -> bool {
    let Some(m) = manager() else { return false };
    // SAFETY: plain call; 1 = radios enabled, 0 = all off.
    unsafe { m.SetSystemRadioState(i32::from(!on)) }.is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mapping Get -> registry must match what `airplane.rs` reads (value 1 = airplane on).
    #[test]
    #[ignore = "talks to the radio management service"]
    fn state_matches_the_registry() {
        let enabled = radios_enabled().expect("IRadioManager answered");
        let reg = crate::platform::windows::registry_watch::read_default_dword(
            r"SYSTEM\CurrentControlSet\Control\RadioManagement\SystemRadioState",
        );
        println!("radios enabled = {enabled}, registry = {reg:?}");
        assert_eq!(reg == Some(1), !enabled);
        // Setting the state it already has is harmless and proves the call works.
        assert!(set_airplane(!enabled));
    }
}
