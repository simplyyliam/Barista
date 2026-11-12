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
const SUB_ENERGYSAVER: GUID = GUID::from_u128(0xde830923_a562_41af_a086_e3a2c6bad2da);
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

/// What an untouched plan uses (checked on Windows 11 with `powercfg /qh`): never when plugged in,
/// 20 % on battery.
const BUILT_IN: Thresholds = (0, 20);

/// A plan only stores a level once somebody changed it; before that the read fails ("file not
/// found") and Windows uses its built-in value, so we do the same.
pub fn read() -> Option<Thresholds> {
    let scheme = active_scheme()?;
    let (mut ac, mut dc) = BUILT_IN;
    // SAFETY: the GUIDs and out pointers live for the calls. A failed read leaves the built-in value.
    unsafe {
        let _ = PowerReadACValueIndex(None, Some(&scheme), Some(&SUB_ENERGYSAVER), Some(&ES_THRESHOLD), &mut ac);
        let _ = PowerReadDCValueIndex(None, Some(&scheme), Some(&SUB_ENERGYSAVER), Some(&ES_THRESHOLD), &mut dc);
    }
    Some((ac, dc))
}

/// Writes both levels to the active plan and re-applies it so Windows re-evaluates right away.
pub fn write((ac, dc): Thresholds) -> bool {
    let Some(scheme) = active_scheme() else { return false };
    // SAFETY: the GUIDs live for the calls.
    unsafe {
        let a = PowerWriteACValueIndex(None, &scheme, Some(&SUB_ENERGYSAVER), Some(&ES_THRESHOLD), ac);
        let d = PowerWriteDCValueIndex(None, &scheme, Some(&SUB_ENERGYSAVER), Some(&ES_THRESHOLD), dc);
        let set = PowerSetActiveScheme(None, Some(&scheme));
        if a != ERROR_SUCCESS || d != 0 || set != ERROR_SUCCESS {
            eprintln!("barista: energy saver level not written (ac {a:?}, dc {d}, apply {set:?})");
        }
        a == ERROR_SUCCESS && d == 0 && set == ERROR_SUCCESS
    }
}

#[cfg(test)]
mod tests {
    /// Reads the real power plan: needs a Windows machine with an energy saver setting.
    #[test]
    #[ignore = "reads the machine's power plan"]
    fn reads_the_active_plan() {
        let levels = super::read();
        println!("levels = {levels:?}");
        assert!(levels.is_some());
        // Writing the same levels back changes nothing, but shows whether the write is allowed.
        assert!(super::write(levels.unwrap()), "write refused");
    }
}
