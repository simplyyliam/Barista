//! Paired Bluetooth devices and whether each is connected, through a `DeviceWatcher`.
//! Windows tells us about added, changed and removed devices; no polling.

use std::collections::HashMap;
use std::sync::Mutex;
use windows::Devices::Enumeration::{
    DeviceInformation, DeviceInformationKind, DeviceInformationUpdate, DeviceWatcher,
};
use windows_collections::{IIterable, IMapView};
use windows::Foundation::{IReference, TypedEventHandler};
use windows::core::HSTRING;
use windows_core::{IInspectable, Interface};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawDevice {
    pub id: String,
    pub name: String,
    pub connected: bool,
}

/// Paired devices of the classic Bluetooth protocol (the GUID is `Aep.ProtocolId` for Bluetooth).
const FILTER: &str = "System.Devices.Aep.ProtocolId:=\"{e0cbf06c-cd8b-4647-bb8a-263b43f0f974}\" AND System.Devices.Aep.IsPaired:=System.StructuredQueryType.Boolean#True";
const CONNECTED: &str = "System.Devices.Aep.IsConnected";

static DEVICES: Mutex<Option<HashMap<String, RawDevice>>> = Mutex::new(None);

fn bool_property(props: &IMapView<HSTRING, IInspectable>) -> Option<bool> {
    let value = props.Lookup(&HSTRING::from(CONNECTED)).ok()?;
    value.cast::<IReference<bool>>().ok()?.Value().ok()
}

fn publish(on_change: fn(Vec<RawDevice>), edit: impl FnOnce(&mut HashMap<String, RawDevice>)) {
    let mut guard = DEVICES.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    edit(map);
    let mut list: Vec<RawDevice> = map.values().cloned().collect();
    drop(guard);
    list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.id.cmp(&b.id)));
    on_change(list);
}

/// The watcher has to stay alive; it is kept here for the life of the app.
static WATCHER: Mutex<Option<DeviceWatcher>> = Mutex::new(None);

pub fn start(on_change: fn(Vec<RawDevice>)) {
    let result = (|| -> windows::core::Result<DeviceWatcher> {
        let props: IIterable<HSTRING> = vec![HSTRING::from(CONNECTED)].into();
        let watcher = DeviceInformation::CreateWatcherWithKindAqsFilterAndAdditionalProperties(
            &HSTRING::from(FILTER),
            &props,
            DeviceInformationKind::AssociationEndpoint,
        )?;
        watcher.Added(&TypedEventHandler::<DeviceWatcher, DeviceInformation>::new(
            move |_, info| {
                if let Some(info) = info.as_ref() {
                    let id = info.Id()?.to_string();
                    let name = info.Name()?.to_string();
                    let connected = bool_property(&info.Properties()?).unwrap_or(false);
                    publish(on_change, |m| {
                        m.insert(id.clone(), RawDevice { id, name, connected });
                    });
                }
                Ok(())
            },
        ))?;
        watcher.Updated(&TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(
            move |_, update| {
                if let Some(u) = update.as_ref() {
                    let id = u.Id()?.to_string();
                    let connected = bool_property(&u.Properties()?);
                    publish(on_change, |m| {
                        if let (Some(d), Some(c)) = (m.get_mut(&id), connected) {
                            d.connected = c;
                        }
                    });
                }
                Ok(())
            },
        ))?;
        watcher.Removed(&TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(
            move |_, update| {
                if let Some(u) = update.as_ref() {
                    let id = u.Id()?.to_string();
                    publish(on_change, |m| {
                        m.remove(&id);
                    });
                }
                Ok(())
            },
        ))?;
        watcher.Start()?;
        Ok(watcher)
    })();
    match result {
        Ok(w) => *WATCHER.lock().unwrap_or_else(|e| e.into_inner()) = Some(w),
        Err(e) => eprintln!("barista: bluetooth devices not watched: {e}"),
    }
}
