//! Bluetooth: the radio on/off state and the paired devices with their connected flag.
//! Switching the radio works; connecting a single device has no public API, so the UI opens
//! Windows' Bluetooth settings for that (ADR 0019).

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothDevice {
    pub id: String,
    pub name: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Bluetooth {
    /// False when there is no Bluetooth radio: the other fields mean nothing then.
    pub available: bool,
    pub enabled: bool,
    /// Windows refused radio access, so the switch can't be used (the UI opens the settings page).
    pub access_denied: bool,
    pub devices: Vec<BluetoothDevice>,
}

pub fn service() -> &'static Service<Bluetooth> {
    static S: OnceLock<Service<Bluetooth>> = OnceLock::new();
    S.get_or_init(|| Service::new(Bluetooth::default()))
}

/// The two inputs the snapshot is built from; either one changing rebuilds it.
#[derive(Default)]
struct Inputs {
    radio: Option<bool>,
    denied: bool,
    devices: Vec<BluetoothDevice>,
}

static INPUTS: Mutex<Inputs> = Mutex::new(Inputs {
    radio: None,
    denied: false,
    devices: Vec::new(),
});

/// Inputs to a snapshot. Devices are only listed while the radio exists.
fn build(radio: Option<bool>, denied: bool, devices: &[BluetoothDevice]) -> Bluetooth {
    match radio {
        None => Bluetooth::default(),
        Some(enabled) => Bluetooth {
            available: true,
            enabled,
            access_denied: denied,
            devices: devices.to_vec(),
        },
    }
}

/// A device paired over classic and Low Energy shows up twice under different ids: list it once, as
/// connected if either side is.
fn merge_by_name(list: Vec<BluetoothDevice>) -> Vec<BluetoothDevice> {
    let mut out: Vec<BluetoothDevice> = Vec::new();
    for d in list {
        match out.iter_mut().find(|o| o.name == d.name) {
            Some(o) => o.connected |= d.connected,
            None => out.push(d),
        }
    }
    out
}

fn publish(edit: impl FnOnce(&mut Inputs)) {
    let mut i = INPUTS.lock().unwrap_or_else(|e| e.into_inner());
    edit(&mut i);
    let snapshot = build(i.radio, i.denied, &i.devices);
    drop(i);
    if super::control_center_active() {
        super::publish_gated(service(), snapshot);
    }
}

#[cfg(windows)]
pub fn radio_changed(r: crate::platform::windows::radios::RadioStates) {
    publish(|i| {
        i.radio = r.bluetooth;
        i.denied = r.denied;
    });
}

#[cfg(windows)]
fn devices_changed(list: Vec<crate::platform::windows::bt_devices::RawDevice>) {
    publish(|i| {
        i.devices = merge_by_name(
            list.into_iter()
                .map(|d| BluetoothDevice {
                    id: d.id,
                    name: d.name,
                    connected: d.connected,
                })
                .collect(),
        );
    });
}

pub fn start() {
    #[cfg(windows)]
    crate::platform::windows::bt_devices::start(devices_changed);
}

pub fn set_enabled(on: bool) {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    crate::platform::windows::radios::set_enabled(crate::platform::windows::radios::Kind::Bluetooth, on);
    #[cfg(not(windows))]
    let _ = on;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(name: &str) -> BluetoothDevice {
        BluetoothDevice {
            id: name.into(),
            name: name.into(),
            connected: false,
        }
    }

    #[test]
    fn a_dual_mode_device_is_listed_once() {
        let mut a = dev("Buds");
        a.id = "classic".into();
        let mut b = dev("Buds");
        b.id = "le".into();
        b.connected = true;
        let merged = merge_by_name(vec![a, b, dev("Mouse")]);
        assert_eq!(merged.len(), 2);
        assert!(merged[0].connected);
    }

    #[test]
    fn no_radio_means_unavailable_and_no_devices() {
        let b = build(None, false, &[dev("Mouse")]);
        assert_eq!(b, Bluetooth::default());
    }

    #[test]
    fn radio_off_still_lists_paired_devices() {
        let b = build(Some(false), false, &[dev("Mouse")]);
        assert!(b.available && !b.enabled);
        assert_eq!(b.devices.len(), 1);
    }
}
