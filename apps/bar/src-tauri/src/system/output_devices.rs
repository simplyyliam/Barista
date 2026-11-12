//! The output devices (speakers, headphones, ...) and which one is the default. Switching uses
//! `IPolicyConfig`, which Windows does not document (ADR 0019); if it is missing, `canSwitch` is
//! false and `setOutputDevice` opens Windows' sound settings instead.

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OutputDevice {
    /// Windows' endpoint id. Pass it back to `setOutputDevice` unchanged.
    pub id: String,
    pub name: String,
    pub default: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OutputDevices {
    pub devices: Vec<OutputDevice>,
    /// True when a click can switch the device. False: `setOutputDevice` opens Windows' picker.
    pub can_switch: bool,
}

pub fn service() -> &'static Service<OutputDevices> {
    static S: OnceLock<Service<OutputDevices>> = OnceLock::new();
    S.get_or_init(|| Service::new(OutputDevices::default()))
}

#[cfg(windows)]
pub fn changed(raw: crate::platform::windows::audio::RawOutputs) {
    if !super::control_center_active() {
        return;
    }
    super::publish_gated(service(), OutputDevices {
        devices: raw
            .devices
            .into_iter()
            .map(|d| OutputDevice {
                id: d.id,
                name: d.name,
                default: d.default,
            })
            .collect(),
        can_switch: raw.can_switch,
    });
}

/// Lists the devices again (the control center came back from `windows` mode).
pub fn refresh() {
    #[cfg(windows)]
    crate::platform::windows::audio::relist_outputs();
}

/// Ids come from Windows (`GetId`): a sane length is all we check here; the audio thread then only
/// accepts an id that is in its fresh device list.
const MAX_ID: usize = 256;

pub fn set_default(id: String) {
    if !super::control_center_active() || id.is_empty() || id.len() > MAX_ID {
        return;
    }
    #[cfg(windows)]
    crate::platform::windows::audio::set_default_output(id);
    #[cfg(not(windows))]
    let _ = id;
}
