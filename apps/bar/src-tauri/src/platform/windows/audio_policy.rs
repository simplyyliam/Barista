//! Switching the default output device. Windows has no public API for it; the sound settings page
//! uses `IPolicyConfig`, an undocumented COM interface that EarTrumpet and SoundSwitch have used
//! since Windows 7 (ADR 0019).
//!
//! We declare only the slots we need to reach `SetDefaultEndpoint`: the vtable order is what
//! matters, the other methods are never called, so their argument types are placeholders.
//! If Windows ever changes the interface, `create()` returns `None` and the bar falls back to
//! opening the sound settings page.

// The COM method names are Windows' own.
#![allow(non_snake_case)]

use core::ffi::c_void;
use windows::core::{interface, IUnknown, IUnknown_Vtbl, GUID, HRESULT, PCWSTR};
use windows::Win32::Media::Audio::{eCommunications, eConsole, eMultimedia, ERole};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

#[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: IUnknown {
    fn GetMixFormat(&self, device: PCWSTR, format: *mut *mut c_void) -> HRESULT;
    fn GetDeviceFormat(&self, device: PCWSTR, default: i32, format: *mut *mut c_void) -> HRESULT;
    fn ResetDeviceFormat(&self, device: PCWSTR) -> HRESULT;
    fn SetDeviceFormat(&self, device: PCWSTR, endpoint: *mut c_void, mix: *mut c_void) -> HRESULT;
    fn GetProcessingPeriod(&self, device: PCWSTR, default: i32, a: *mut i64, b: *mut i64) -> HRESULT;
    fn SetProcessingPeriod(&self, device: PCWSTR, period: *mut i64) -> HRESULT;
    fn GetShareMode(&self, device: PCWSTR, mode: *mut c_void) -> HRESULT;
    fn SetShareMode(&self, device: PCWSTR, mode: *mut c_void) -> HRESULT;
    fn GetPropertyValue(&self, device: PCWSTR, fx: i32, key: *const c_void, value: *mut c_void) -> HRESULT;
    fn SetPropertyValue(&self, device: PCWSTR, fx: i32, key: *const c_void, value: *const c_void) -> HRESULT;
    fn SetDefaultEndpoint(&self, device: PCWSTR, role: ERole) -> HRESULT;
    fn SetEndpointVisibility(&self, device: PCWSTR, visible: i32) -> HRESULT;
}

/// `CPolicyConfigClient`.
const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

/// A handle to the interface. Create it on the audio thread (it needs COM there).
pub struct PolicyConfig(IPolicyConfig);

/// `None` when Windows no longer offers the interface.
pub fn create() -> Option<PolicyConfig> {
    // SAFETY: plain COM creation; QueryInterface fails cleanly if the IID is gone.
    unsafe {
        CoCreateInstance::<_, IPolicyConfig>(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)
            .ok()
            .map(PolicyConfig)
    }
}

impl PolicyConfig {
    /// Makes `id` (an endpoint id from the current device list) the default for every role.
    pub fn set_default(&self, id: &str) -> bool {
        let mut wide: Vec<u16> = id.encode_utf16().collect();
        wide.push(0);
        let device = PCWSTR(wide.as_ptr());
        [eConsole, eMultimedia, eCommunications].into_iter().all(|role| {
            // SAFETY: `wide` outlives the calls; the id was taken from Windows' own list.
            unsafe { self.0.SetDefaultEndpoint(device, role) }.is_ok()
        })
    }
}
