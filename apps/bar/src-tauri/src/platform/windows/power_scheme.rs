//! Energy saver ("battery saver") switch. Windows has no API to turn it on; Quick Settings flips
//! the charge level at which it starts by itself (power setting `ESBATTTHRESHOLD`). 100 means
//! "on now", 0 means "never". We write that value to the active power plan (ADR 0020).

use windows::core::GUID;
use windows::Win32::Foundation::{LocalFree, ERROR_SUCCESS, HLOCAL};
use windows::Win32::System::Power::{
    PowerGetActiveScheme, PowerReadACValueIndex, PowerReadDCValueIndex, PowerSetActiveScheme,
    PowerWriteACValueIndex, PowerWriteDCValueIndex,
};

/// `SUB_ENERGYSAVER`.
const SUB_ENERGYSAVER: GUID = GUID::from_u128(0xde830923_a562_41af_9685_e3a2c6bad2da);
/// `ESBATTTHRESHOLD`.
const ES_THRESHOLD: GUID = GUID::from_u128(0xe69653ca_cf7f_4f05_aa73_cb833fa90ad4);

/// The charge levels (plugged in, on battery) at which energy saver starts.
pub type Thresholds = (u32, u32);

fn active_scheme() -> Option<GUID> {
    let mut p: *mut GUID = std::ptr::null_mut();
    // SAFETY: Windows fills `p` with a GUID it allocated; we copy it and free it right away.
    unsafe {
        if PowerGetActiveScheme(None, &mut p) != ERROR_SUCCESS || p.is_null() {
            return None;
        }
        let guid = *p;
        let _ = LocalFree(Some(HLOCAL(p.cast())));
        Some(guid)
    }
}

pub fn read() -> Option<Thresholds> {
    let scheme = active_scheme()?;
    let (mut ac, mut dc) = (0u32, 0u32);
    // SAFETY: the GUIDs and out pointers live for the calls.
    unsafe {
        let a = PowerReadACValueIndex(None, Some(&scheme), Some(&SUB_ENERGYSAVER), Some(&ES_THRESHOLD), &mut ac);
        let d = PowerReadDCValueIndex(None, Some(&scheme), Some(&SUB_ENERGYSAVER), Some(&ES_THRESHOLD), &mut dc);
        (a == ERROR_SUCCESS && d == 0).then_some((ac, dc))
    }
}

/// Writes both levels to the active plan and re-applies it so Windows re-evaluates right away.
pub fn write((ac, dc): Thresholds) -> bool {
    let Some(scheme) = active_scheme() else { return false };
    // SAFETY: the GUIDs live for the calls.
    unsafe {
        let a = PowerWriteACValueIndex(None, &scheme, Some(&SUB_ENERGYSAVER), Some(&ES_THRESHOLD), ac);
        let d = PowerWriteDCValueIndex(None, &scheme, Some(&SUB_ENERGYSAVER), Some(&ES_THRESHOLD), dc);
        a == ERROR_SUCCESS && d == 0 && PowerSetActiveScheme(None, Some(&scheme)) == ERROR_SUCCESS
    }
}
