//! Wi-Fi and Bluetooth radios through `Windows.Devices.Radios`: read the on/off state, switch it,
//! and hear about changes (also from Windows' own Quick Settings or a hardware switch).
//!
//! One thread owns the WinRT objects. `StateChanged` callbacks run on Windows' threads and only
//! send a message to it; it re-reads everything. Nothing polls.

use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use windows::Devices::Radios::{Radio, RadioAccessStatus, RadioKind, RadioState};
use windows::Foundation::TypedEventHandler;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows_core::IInspectable;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Wifi,
    Bluetooth,
}

/// `None` = this machine has no such radio.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RadioStates {
    pub wifi: Option<bool>,
    pub bluetooth: Option<bool>,
    /// Windows refused radio access (privacy setting): we can read, but not switch.
    pub denied: bool,
}

enum Msg {
    Refresh,
    Set(Kind, bool),
}

static TX: OnceLock<Sender<Msg>> = OnceLock::new();

/// Switches a radio. The change comes back through `on_change`.
pub fn set_enabled(kind: Kind, on: bool) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::Set(kind, on));
    }
}

/// Reads again (the radio list itself has no change event, so the panel opening asks).
pub fn refresh() {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::Refresh);
    }
}

/// A radio with our `StateChanged` hook, removed again on drop.
struct Hooked {
    radio: Radio,
    token: i64,
}

impl Drop for Hooked {
    fn drop(&mut self) {
        let _ = self.radio.RemoveStateChanged(self.token);
    }
}

fn read(radios: &[Hooked], denied: bool) -> RadioStates {
    let state_of = |kind: RadioKind| {
        let mut found = None;
        for h in radios {
            if h.radio.Kind().ok() == Some(kind) {
                let on = h.radio.State().ok() == Some(RadioState::On);
                found = Some(found.unwrap_or(false) || on);
            }
        }
        found
    };
    RadioStates {
        wifi: state_of(RadioKind::WiFi),
        bluetooth: state_of(RadioKind::Bluetooth),
        denied,
    }
}

fn hook_all(tx: &Sender<Msg>) -> Vec<Hooked> {
    let Ok(list) = Radio::GetRadiosAsync().and_then(|op| op.join()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for radio in &list {
        let tx = tx.clone();
        let handler = TypedEventHandler::<Radio, IInspectable>::new(move |_, _| {
            let _ = tx.send(Msg::Refresh);
            Ok(())
        });
        if let Ok(token) = radio.StateChanged(&handler) {
            out.push(Hooked { radio, token });
        }
    }
    out
}

pub fn start(on_change: fn(RadioStates)) {
    let (tx, rx) = channel();
    if TX.set(tx.clone()).is_err() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("barista-radios".into())
        .spawn(move || {
            // SAFETY: plain COM init for this thread; the WinRT calls below need an apartment.
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
            }
            // Asked once; a desktop app normally gets `Allowed` without a prompt.
            let denied = Radio::RequestAccessAsync()
                .and_then(|op| op.join())
                .map(|s| s != RadioAccessStatus::Allowed)
                .unwrap_or(true);
            let mut radios = hook_all(&tx);
            on_change(read(&radios, denied));
            while let Ok(msg) = rx.recv() {
                if let Msg::Set(kind, on) = msg {
                    let wanted = if on { RadioState::On } else { RadioState::Off };
                    let want_kind = match kind {
                        Kind::Wifi => RadioKind::WiFi,
                        Kind::Bluetooth => RadioKind::Bluetooth,
                    };
                    for h in &radios {
                        if h.radio.Kind().ok() == Some(want_kind) {
                            let _ = h.radio.SetStateAsync(wanted).and_then(|op| op.join());
                        }
                    }
                }
                // The radio list can change (a USB adapter): hook fresh every time. Cheap.
                drop(radios);
                radios = hook_all(&tx);
                on_change(read(&radios, denied));
            }
        });
    if let Err(e) = spawned {
        eprintln!("barista: could not start the radios thread: {e}");
    }
}
