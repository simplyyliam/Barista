//! Switching energy saver ("battery saver"). Windows has no public API for it. Quick Settings
//! publishes to an undocumented Windows notification state (WNF), and the power service then flips
//! its switch and records it in `EnergySaverState` (ADR 0020). We publish the same state, the way
//! the open source G-Helper does: 1 = on, 2 = off, no administrator rights needed.
//!
//! The state name belongs to Windows 11 (checked on build 26300). If a build stops accepting it,
//! `publish` returns an error code and the tile does nothing; the bar never falls back to a link.

use core::ffi::c_void;

/// The WNF state that Quick Settings' energy saver tile publishes to.
const ENERGY_SAVER_STATE: u64 = 0x41C6_013D_A3BC_3075;

#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlPublishWnfStateData(
        state_name: u64,
        type_id: *const c_void,
        buffer: *const c_void,
        length: u32,
        explicit_scope: *const c_void,
    ) -> i32;
}

/// What is published for on and off.
pub fn value(on: bool) -> u32 {
    if on { 1 } else { 2 }
}

/// Switches energy saver. `Err` carries the NTSTATUS Windows answered with.
pub fn publish(on: bool) -> Result<(), i32> {
    let data = value(on);
    // SAFETY: the four bytes of `data` live for the call and `length` is their size; the type id
    // and scope are null, which means "none".
    let status = unsafe {
        RtlPublishWnfStateData(
            ENERGY_SAVER_STATE,
            core::ptr::null(),
            core::ptr::from_ref(&data).cast(),
            size_of::<u32>() as u32,
            core::ptr::null(),
        )
    };
    if status >= 0 { Ok(()) } else { Err(status) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_is_one_and_off_is_two() {
        assert_eq!(value(true), 1);
        assert_eq!(value(false), 2);
    }

    /// Switches the real energy saver off and on again and checks Windows' own record follows.
    /// Leaves it as it found it.
    #[test]
    #[ignore = "switches energy saver for a moment"]
    fn windows_follows_the_published_state() {
        use crate::platform::windows::registry_watch::read_machine_dword;
        let read = || read_machine_dword(r"SYSTEM\CurrentControlSet\Control\Power", "EnergySaverState");
        let original = read();
        let settle = || std::thread::sleep(std::time::Duration::from_millis(1200));
        publish(false).expect("publish off");
        settle();
        let after_off = read();
        publish(true).expect("publish on");
        settle();
        let after_on = read();
        // Put it back the way it was.
        if original == Some(2) {
            publish(false).expect("restore");
        }
        println!("original {original:?}, after off {after_off:?}, after on {after_on:?}");
        assert_eq!(after_off, Some(2));
        assert_eq!(after_on, Some(1));
    }
}
