//! Wi-Fi: on/off, the network we are on and the networks in range. Joining a new secured network
//! is out of scope: the UI opens Windows' own Wi-Fi flyout for that (ADR 0019).

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::{Mutex, OnceLock};

/// The list the UI gets is short; Windows can report dozens of neighbours.
const MAX_NETWORKS: usize = 30;
/// An SSID is at most 32 bytes.
const MAX_SSID: usize = 32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WifiNetwork {
    pub ssid: String,
    /// 0 to 100.
    pub signal: u8,
    pub secured: bool,
    /// Windows has a saved profile, so `connectWifi` can join it.
    pub known: bool,
    pub connected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Wifi {
    /// False when there is no Wi-Fi adapter: the other fields mean nothing then.
    pub available: bool,
    pub enabled: bool,
    /// Windows refused radio access, so the switch can't be used (the UI opens the settings page).
    pub access_denied: bool,
    /// False when Windows hides network names until the user allows location access
    /// (`openSettings("location")`).
    pub location_allowed: bool,
    /// The SSID we are connected to.
    pub current: Option<String>,
    /// Connected first, then by signal.
    pub networks: Vec<WifiNetwork>,
}

pub fn service() -> &'static Service<Wifi> {
    static S: OnceLock<Service<Wifi>> = OnceLock::new();
    S.get_or_init(|| Service::new(Wifi::default()))
}

/// One entry per name (the strongest wins, a connected one always wins), hidden networks dropped,
/// connected first, then strongest, capped.
pub fn clean_networks(raw: &[RawNetwork]) -> Vec<WifiNetwork> {
    let mut out: Vec<WifiNetwork> = Vec::new();
    for n in raw.iter().filter(|n| !n.ssid.trim().is_empty()) {
        let item = WifiNetwork {
            ssid: n.ssid.clone(),
            signal: n.signal,
            secured: n.secured,
            known: n.profile.is_some(),
            connected: n.connected,
        };
        match out.iter_mut().find(|o| o.ssid == item.ssid) {
            Some(old) => {
                let better = (item.connected, item.signal) > (old.connected, old.signal);
                let known = old.known || item.known;
                if better {
                    *old = item;
                }
                old.known = known;
            }
            None => out.push(item),
        }
    }
    out.sort_by(|a, b| {
        b.connected
            .cmp(&a.connected)
            .then(b.signal.cmp(&a.signal))
            .then(a.ssid.cmp(&b.ssid))
    });
    out.truncate(MAX_NETWORKS);
    out
}

#[cfg(windows)]
use crate::platform::windows::wlan::RawNetwork;
#[cfg(not(windows))]
#[derive(Clone)]
pub struct RawNetwork {
    pub ssid: String,
    pub signal: u8,
    pub secured: bool,
    pub connected: bool,
    pub profile: Option<String>,
}

/// What the snapshot is built from. Either source changing rebuilds it.
struct Inputs {
    /// `None` = no Wi-Fi radio.
    radio: Option<bool>,
    denied: bool,
    has_interface: bool,
    location_denied: bool,
    networks: Vec<RawNetwork>,
}

static INPUTS: Mutex<Inputs> = Mutex::new(Inputs {
    radio: None,
    denied: false,
    has_interface: false,
    location_denied: false,
    networks: Vec::new(),
});

fn build(i: &Inputs) -> Wifi {
    if !i.has_interface && i.radio.is_none() {
        return Wifi::default();
    }
    // Windows can show an adapter without a radio entry: then it is simply on.
    let enabled = i.radio.unwrap_or(true);
    let networks = if enabled { clean_networks(&i.networks) } else { Vec::new() };
    Wifi {
        available: true,
        enabled,
        access_denied: i.denied,
        location_allowed: !i.location_denied,
        current: networks.iter().find(|n| n.connected).map(|n| n.ssid.clone()),
        networks,
    }
}

fn publish(edit: impl FnOnce(&mut Inputs)) {
    let mut i = INPUTS.lock().unwrap_or_else(|e| e.into_inner());
    edit(&mut i);
    let snapshot = build(&i);
    drop(i);
    if super::control_center_active() {
        service().set(snapshot);
    }
}

#[cfg(windows)]
pub fn radio_changed(r: crate::platform::windows::radios::RadioStates) {
    publish(|i| {
        i.radio = r.wifi;
        i.denied = r.denied;
    });
}

#[cfg(windows)]
fn wlan_changed(raw: crate::platform::windows::wlan::RawWifi) {
    publish(|i| {
        i.has_interface = raw.has_interface;
        i.location_denied = raw.location_denied;
        i.networks = raw.networks;
    });
}

pub fn start() {
    #[cfg(windows)]
    crate::platform::windows::wlan::start(wlan_changed);
}

pub fn set_enabled(on: bool) {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    crate::platform::windows::radios::set_enabled(crate::platform::windows::radios::Kind::Wifi, on);
    #[cfg(not(windows))]
    let _ = on;
}

pub fn scan() {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    crate::platform::windows::wlan::scan();
}

pub fn disconnect() {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    crate::platform::windows::wlan::disconnect();
}

/// Joins a network Windows already knows. Anything else (new, unknown name, too long) opens
/// Windows' own Wi-Fi flyout: joining a new secured network is out of scope.
pub fn connect(ssid: &str) {
    if !super::control_center_active() {
        return;
    }
    let profile = if ssid.len() <= MAX_SSID {
        let i = INPUTS.lock().unwrap_or_else(|e| e.into_inner());
        i.networks
            .iter()
            .filter(|n| n.ssid == ssid)
            .find_map(|n| n.profile.clone())
    } else {
        None
    };
    #[cfg(windows)]
    match profile {
        Some(p) => crate::platform::windows::wlan::connect(p),
        None => crate::platform::windows::shell::open_uri("ms-availablenetworks:"),
    }
    #[cfg(not(windows))]
    let _ = profile;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(ssid: &str, signal: u8, connected: bool, profile: bool) -> RawNetwork {
        RawNetwork {
            ssid: ssid.into(),
            signal,
            secured: true,
            connected,
            profile: profile.then(|| ssid.into()),
        }
    }

    #[test]
    fn hidden_networks_are_dropped_and_names_are_unique() {
        let list = clean_networks(&[
            net("", 90, false, false),
            net("Home", 40, false, false),
            net("Home", 70, false, true),
        ]);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].signal, 70);
        assert!(list[0].known);
    }

    #[test]
    fn connected_comes_first_then_signal() {
        let list = clean_networks(&[
            net("Loud", 95, false, false),
            net("Mine", 30, true, true),
            net("Quiet", 10, false, false),
        ]);
        let names: Vec<_> = list.iter().map(|n| n.ssid.as_str()).collect();
        assert_eq!(names, ["Mine", "Loud", "Quiet"]);
    }

    #[test]
    fn the_list_is_capped() {
        let raw: Vec<_> = (0..100).map(|i| net(&format!("n{i}"), 50, false, false)).collect();
        assert_eq!(clean_networks(&raw).len(), MAX_NETWORKS);
    }

    #[test]
    fn radio_off_shows_no_networks() {
        let i = Inputs {
            radio: Some(false),
            denied: false,
            has_interface: true,
            location_denied: false,
            networks: vec![net("Home", 50, true, true)],
        };
        let w = build(&i);
        assert!(w.available && !w.enabled);
        assert!(w.networks.is_empty() && w.current.is_none());
    }

    #[test]
    fn no_adapter_means_unavailable() {
        let i = Inputs {
            radio: None,
            denied: false,
            has_interface: false,
            location_denied: false,
            networks: Vec::new(),
        };
        assert_eq!(build(&i), Wifi::default());
    }
}
