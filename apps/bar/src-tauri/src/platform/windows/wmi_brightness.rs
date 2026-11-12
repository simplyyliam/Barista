//! The laptop screen's brightness through WMI (`WmiMonitorBrightness` in `ROOT\WMI`). Only laptop
//! panels have it: a desktop has no such class and everything here returns `None`.
//!
//! Changes made elsewhere (the brightness keys, Windows settings) arrive as
//! `WmiMonitorBrightnessEvent`: a thread blocks in `Next` until one comes. Nothing polls.

use windows::core::{w, BSTR, HSTRING, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoSetProxyBlanket, CLSCTX_INPROC_SERVER, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL,
    RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemContext, IWbemLocator, IWbemServices, WbemLocator,
    WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_GENERIC_FLAG_TYPE,
};

/// `RPC_C_AUTHN_WINNT`: the usual login for local WMI.
const AUTHN_WINNT: u32 = 10;

const QUERY_FLAGS: WBEM_GENERIC_FLAG_TYPE =
    WBEM_GENERIC_FLAG_TYPE(WBEM_FLAG_FORWARD_ONLY.0 | WBEM_FLAG_RETURN_IMMEDIATELY.0);

/// A connection to `ROOT\WMI`. Use it on the thread that made it.
pub struct Wmi(IWbemServices);

pub fn connect() -> Option<Wmi> {
    // SAFETY: plain COM calls; the thread has COM initialised (the caller's job).
    unsafe {
        let locator: IWbemLocator =
            CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER).ok()?;
        let services = locator
            .ConnectServer(
                &BSTR::from(r"ROOT\WMI"),
                &BSTR::new(),
                &BSTR::new(),
                &BSTR::new(),
                0,
                &BSTR::new(),
                None::<&IWbemContext>,
            )
            .ok()?;
        CoSetProxyBlanket(
            &services,
            AUTHN_WINNT,
            0,
            PCWSTR::null(),
            RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
        )
        .ok()?;
        Some(Wmi(services))
    }
}

fn get_variant(obj: &IWbemClassObject, name: &str) -> Option<VARIANT> {
    let mut v = VARIANT::default();
    // SAFETY: `v` lives for the call.
    unsafe { obj.Get(&HSTRING::from(name), 0, &mut v, None, None) }.ok()?;
    Some(v)
}

impl Wmi {
    /// All objects of a query, in order. `timeout` -1 waits forever (events).
    fn first(&self, query: &str) -> Option<IWbemClassObject> {
        // SAFETY: plain COM calls; the object array holds one slot.
        unsafe {
            let rows = self
                .0
                .ExecQuery(&BSTR::from("WQL"), &BSTR::from(query), QUERY_FLAGS, None::<&IWbemContext>)
                .ok()?;
            let mut objects = [None];
            let mut returned = 0u32;
            let _ = rows.Next(5000, &mut objects, &mut returned);
            if returned == 0 {
                return None;
            }
            objects[0].take()
        }
    }

    /// The panel's brightness, 0 to 100, and its PnP id (`BOE0A1B`, the middle part of WMI's
    /// `DISPLAY\BOE0A1B\...` instance name). `None` without a laptop panel.
    pub fn read_with_id(&self) -> Option<(u8, String)> {
        let obj = self.first(
            "SELECT InstanceName, CurrentBrightness FROM WmiMonitorBrightness WHERE Active=TRUE",
        )?;
        let level = i32::try_from(&get_variant(&obj, "CurrentBrightness")?).ok()?;
        let pnp = get_variant(&obj, "InstanceName")
            .and_then(|v| BSTR::try_from(&v).ok())
            .and_then(|s| s.to_string().split(super::ddc::PATH_SEP).nth(1).map(str::to_owned))
            .unwrap_or_default();
        Some((level.clamp(0, 100) as u8, pnp))
    }

    #[cfg(test)]
    pub fn read(&self) -> Option<u8> {
        self.read_with_id().map(|(level, _)| level)
    }

    /// Sets the panel brightness (0 to 100). Returns whether Windows accepted it.
    pub fn set(&self, percent: u8) -> bool {
        let Some(obj) = self.first("SELECT * FROM WmiMonitorBrightnessMethods WHERE Active=TRUE") else {
            return false;
        };
        let Some(path) = get_variant(&obj, "__PATH").and_then(|v| BSTR::try_from(&v).ok()) else {
            return false;
        };
        // SAFETY: plain COM calls; every out pointer lives for its call.
        unsafe {
            let mut class = None;
            if self
                .0
                .GetObject(
                    &BSTR::from("WmiMonitorBrightnessMethods"),
                    WBEM_GENERIC_FLAG_TYPE(0),
                    None::<&IWbemContext>,
                    Some(&mut class),
                    None,
                )
                .is_err()
            {
                return false;
            }
            let Some(class) = class else { return false };
            let (mut in_sig, mut out_sig) = (None, None);
            if class
                .GetMethod(w!("WmiSetBrightness"), 0, &mut in_sig, &mut out_sig)
                .is_err()
            {
                return false;
            }
            let Some(Ok(params)) = in_sig.map(|s| s.SpawnInstance(0)) else {
                return false;
            };
            // Timeout 1 = "now"; the brightness is the percentage.
            if params.Put(w!("Timeout"), 0, &VARIANT::from(1i32), 0).is_err()
                || params.Put(w!("Brightness"), 0, &VARIANT::from(i32::from(percent)), 0).is_err()
            {
                return false;
            }
            self.0
                .ExecMethod(
                    &path,
                    &BSTR::from("WmiSetBrightness"),
                    WBEM_GENERIC_FLAG_TYPE(0),
                    None::<&IWbemContext>,
                    &params,
                    None,
                    None,
                )
                .is_ok()
        }
    }
}

/// Blocks until the brightness changes somewhere else, then calls `on_event`; repeats forever.
/// Run it on its own thread (with COM initialised). Returns when the class does not exist (a
/// desktop). If WMI drops an established subscription (service restart, resume), it subscribes
/// again after a pause: one retry per failure, never a loop of polls.
pub fn watch(on_event: impl Fn()) {
    while watch_once(&on_event) {
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}

/// Returns true if a subscription worked and then ended (worth retrying).
fn watch_once(on_event: &impl Fn()) -> bool {
    let Some(wmi) = connect() else { return false };
    // SAFETY: plain COM calls on a live connection.
    unsafe {
        let Ok(events) = wmi.0.ExecNotificationQuery(
            &BSTR::from("WQL"),
            &BSTR::from("SELECT * FROM WmiMonitorBrightnessEvent"),
            QUERY_FLAGS,
            None::<&IWbemContext>,
        ) else {
            return false;
        };
        loop {
            let mut objects = [None];
            let mut returned = 0u32;
            // -1 = WBEM_INFINITE: the thread sleeps here until Windows has an event.
            let hr = events.Next(-1, &mut objects, &mut returned);
            if hr.is_err() {
                return true;
            }
            if returned > 0 {
                on_event();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

    #[test]
    #[ignore = "needs a laptop with a built-in panel"]
    fn reads_the_laptop_brightness() {
        // SAFETY: first COM call on this test thread.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let wmi = connect().expect("ROOT\\WMI");
        let level = wmi.read();
        println!("brightness={level:?}");
        assert!(level.is_some());
    }

    /// Sets the same value it just read, so the screen does not change.
    #[test]
    #[ignore = "needs a laptop with a built-in panel"]
    fn sets_the_laptop_brightness_to_itself() {
        // SAFETY: as above.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let wmi = connect().expect("ROOT\\WMI");
        let level = wmi.read().expect("a panel");
        assert!(wmi.set(level));
        assert_eq!(wmi.read(), Some(level));
    }
}
