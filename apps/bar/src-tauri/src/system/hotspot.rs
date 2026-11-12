//! Mobile hotspot: on or off, connected devices. Windows has no hotspot state event, so this reads
//! at start, after our own start or stop, when the connection changes and when the control center
//! opens (ADR 0019).

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum HotspotState {
    #[default]
    Off,
    On,
    /// Starting or stopping (takes a few seconds): show a spinner, ignore clicks.
    Changing,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Hotspot {
    /// False when this PC can't share its connection (no internet connection, no capable adapter,
    /// blocked by the organisation): hide the tile or open `openSettings("hotspot")`.
    pub supported: bool,
    pub state: HotspotState,
    pub clients: u32,
    pub max_clients: u32,
}

pub fn service() -> &'static Service<Hotspot> {
    static S: OnceLock<Service<Hotspot>> = OnceLock::new();
    S.get_or_init(|| Service::new(Hotspot::default()))
}

#[cfg(windows)]
fn changed(raw: crate::platform::windows::tethering::RawHotspot) {
    use crate::platform::windows::tethering::RawState;
    if !super::control_center_active() {
        return;
    }
    service().set(Hotspot {
        supported: raw.supported,
        state: match raw.state {
            RawState::Off => HotspotState::Off,
            RawState::On => HotspotState::On,
            RawState::Changing => HotspotState::Changing,
        },
        clients: raw.clients,
        max_clients: raw.max_clients,
    });
}

pub fn start() {
    #[cfg(windows)]
    crate::platform::windows::tethering::start(changed);
}

pub fn refresh() {
    #[cfg(windows)]
    crate::platform::windows::tethering::refresh();
}

pub fn set_enabled(on: bool) {
    #[cfg(windows)]
    crate::platform::windows::tethering::set_enabled(on);
    #[cfg(not(windows))]
    let _ = on;
}
