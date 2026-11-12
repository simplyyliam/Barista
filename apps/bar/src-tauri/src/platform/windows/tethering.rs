//! Mobile hotspot through `NetworkOperatorTetheringManager`. It has no state event, so we read at
//! start, after our own start or stop, when the network connection changes, and when the control
//! center opens. Nothing polls.

use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use windows::Networking::Connectivity::{NetworkInformation, NetworkStatusChangedEventHandler};
use windows::Networking::NetworkOperators::{
    NetworkOperatorTetheringManager, TetheringCapability, TetheringOperationalState,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RawState {
    #[default]
    Off,
    On,
    Changing,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RawHotspot {
    pub supported: bool,
    pub state: RawState,
    pub clients: u32,
    pub max_clients: u32,
}

enum Msg {
    Refresh,
    Set(bool),
}

static TX: OnceLock<Sender<Msg>> = OnceLock::new();

pub fn refresh() {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::Refresh);
    }
}

pub fn set_enabled(on: bool) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::Set(on));
    }
}

/// The manager for the connection the PC is using for the internet (a hotspot shares that one).
fn manager() -> Option<NetworkOperatorTetheringManager> {
    let profile = NetworkInformation::GetInternetConnectionProfile().ok()?;
    if NetworkOperatorTetheringManager::GetTetheringCapabilityFromConnectionProfile(&profile).ok()?
        != TetheringCapability::Enabled
    {
        return None;
    }
    NetworkOperatorTetheringManager::CreateFromConnectionProfile(&profile).ok()
}

fn read() -> RawHotspot {
    let Some(m) = manager() else {
        return RawHotspot::default();
    };
    RawHotspot {
        supported: true,
        state: match m.TetheringOperationalState() {
            Ok(TetheringOperationalState::On) => RawState::On,
            Ok(TetheringOperationalState::InTransition) => RawState::Changing,
            _ => RawState::Off,
        },
        clients: m.ClientCount().unwrap_or(0),
        max_clients: m.MaxClientCount().unwrap_or(0),
    }
}

pub fn start(on_change: fn(RawHotspot)) {
    let (tx, rx) = channel();
    if TX.set(tx.clone()).is_err() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("barista-hotspot".into())
        .spawn(move || {
            // SAFETY: plain COM init for this thread; the WinRT calls below need an apartment.
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
            }
            // The connection changing (Wi-Fi joined, cable plugged) can make the hotspot possible or not.
            let tx2 = tx.clone();
            let handler = NetworkStatusChangedEventHandler::new(move |_| {
                let _ = tx2.send(Msg::Refresh);
                Ok(())
            });
            let token = NetworkInformation::NetworkStatusChanged(&handler).ok();
            on_change(read());
            while let Ok(msg) = rx.recv() {
                if let Msg::Set(on) = msg {
                    if let Some(m) = manager() {
                        // Takes a few seconds, and this thread waits: say "changing" first.
                        on_change(RawHotspot {
                            state: RawState::Changing,
                            ..read()
                        });
                        let op = if on { m.StartTetheringAsync() } else { m.StopTetheringAsync() };
                        let _ = op.and_then(|op| op.join());
                    }
                }
                on_change(read());
            }
            if let Some(t) = token {
                let _ = NetworkInformation::RemoveNetworkStatusChanged(t);
            }
        });
    if let Err(e) = spawned {
        eprintln!("barista: could not start the hotspot thread: {e}");
    }
}
