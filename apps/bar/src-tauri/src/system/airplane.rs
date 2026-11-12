//! Airplane mode: the real state, read live. Windows has no public way to switch it, so the UI
//! opens its settings page instead (`openSettings("airplaneMode")`, ADR 0019).

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AirplaneMode {
    /// False when Windows has no radio management here (a desktop PC): `on` means nothing then.
    pub supported: bool,
    pub on: bool,
}

pub fn service() -> &'static Service<AirplaneMode> {
    static S: OnceLock<Service<AirplaneMode>> = OnceLock::new();
    S.get_or_init(|| Service::new(AirplaneMode::default()))
}

/// `SystemRadioState` is 1 when radios are switched off (airplane mode on), 0 when they are on.
pub fn from_registry(value: Option<u32>) -> AirplaneMode {
    match value {
        None => AirplaneMode::default(),
        Some(v) => AirplaneMode {
            supported: true,
            on: v == 1,
        },
    }
}

#[cfg(windows)]
const KEY: &str = r"SYSTEM\CurrentControlSet\Control\RadioManagement\SystemRadioState";

pub fn refresh() {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    service().set(from_registry(crate::platform::windows::registry_watch::read_default_dword(KEY)));
}

pub fn start() {
    refresh();
    #[cfg(windows)]
    {
        // Watch the parent: the value sits in the default value of a child key, and a change there
        // reaches a subtree watch.
        let parent = r"SYSTEM\CurrentControlSet\Control\RadioManagement";
        if !crate::platform::windows::registry_watch::watch(parent, refresh) {
            eprintln!("barista: airplane mode not watched");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_value_maps_to_state() {
        assert_eq!(from_registry(None), AirplaneMode::default());
        assert_eq!(from_registry(Some(1)), AirplaneMode { supported: true, on: true });
        assert_eq!(from_registry(Some(0)), AirplaneMode { supported: true, on: false });
    }
}
