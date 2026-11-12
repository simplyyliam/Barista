//! Puts the laptop panel (WMI) and the external monitors (DDC/CI) behind one worker thread.
//!
//! DDC can be slow, so everything slow happens here and never on the UI's call. Messages:
//! "read everything" (start, monitors plugged, panel opened), "read the laptop panel" (WMI told us
//! the keys were pressed) and "set one display". A burst of sets (a slider) keeps only the last
//! value per display. No polling: the WMI event thread and the app's own events wake us.

use super::{ddc, wmi_brightness};
use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

pub const INTERNAL_ID: &str = "wmi:internal";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawDisplay {
    pub id: String,
    pub name: String,
    pub internal: bool,
    /// 0 to 100; `None` = the display did not answer (or too slowly): no control.
    pub percent: Option<u8>,
}

enum Msg {
    RefreshAll,
    RefreshInternal,
    Set(String, u8),
}

static TX: OnceLock<Sender<Msg>> = OnceLock::new();

pub fn refresh_all() {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::RefreshAll);
    }
}

pub fn set(id: String, percent: u8) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::Set(id, percent));
    }
}

fn publish(
    internal: &Option<(u8, String)>,
    external: &[ddc::RawDdc],
    on_change: fn(Vec<RawDisplay>),
) {
    let mut list = Vec::new();
    if let Some((percent, _)) = internal {
        list.push(RawDisplay {
            id: INTERNAL_ID.into(),
            name: "Built-in display".into(),
            internal: true,
            percent: Some(*percent),
        });
    }
    list.extend(external.iter().map(|d| RawDisplay {
        id: d.id.clone(),
        name: d.name.clone(),
        internal: false,
        percent: d.percent,
    }));
    on_change(list);
}

pub fn start(on_change: fn(Vec<RawDisplay>)) {
    let (tx, rx) = channel();
    if TX.set(tx.clone()).is_err() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("barista-brightness".into())
        .spawn(move || {
            // SAFETY: plain COM init for this thread; WMI needs it.
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
            }
            let wmi = wmi_brightness::connect();
            // The brightness keys and Windows settings: blocks in WMI until one happens.
            let event_tx = tx.clone();
            let _ = std::thread::Builder::new()
                .name("barista-brightness-events".into())
                .spawn(move || {
                    // SAFETY: as above, for this thread.
                    unsafe {
                        let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
                    }
                    wmi_brightness::watch(|| {
                        let _ = event_tx.send(Msg::RefreshInternal);
                    });
                });

            let mut internal: Option<(u8, String)> = None;
            let mut external: Vec<ddc::RawDdc> = Vec::new();
            let mut msg = Some(Msg::RefreshAll);
            while let Some(first) = msg.take().or_else(|| rx.recv().ok()) {
                // Take everything queued: a slider's burst becomes one pass, last value wins.
                let mut sets: Vec<(String, u8)> = Vec::new();
                let (mut all, mut inner) = (false, false);
                for m in std::iter::once(first).chain(std::iter::from_fn(|| rx.try_recv().ok())) {
                    match m {
                        Msg::RefreshAll => all = true,
                        Msg::RefreshInternal => inner = true,
                        Msg::Set(id, p) => match sets.iter_mut().find(|(i, _)| *i == id) {
                            Some(s) => s.1 = p,
                            None => sets.push((id, p)),
                        },
                    }
                }
                for (id, percent) in sets {
                    if id == INTERNAL_ID {
                        if let Some(w) = &wmi {
                            if w.set(percent) {
                                if let Some((p, _)) = internal.as_mut() {
                                    *p = percent;
                                }
                            }
                        }
                    } else if ddc::set(&id, percent) {
                        // Trust the monitor: reading back over the cable is slow.
                        if let Some(d) = external.iter_mut().find(|d| d.id == id) {
                            d.percent = Some(percent);
                        }
                    }
                }
                if all || inner {
                    internal = wmi.as_ref().and_then(|w| w.read_with_id());
                }
                if all {
                    let skip = internal.as_ref().map(|(_, pnp)| pnp.as_str());
                    external = ddc::read_all(skip);
                }
                publish(&internal, &external, on_change);
            }
        });
    if let Err(e) = spawned {
        eprintln!("barista: could not start the brightness thread: {e}");
    }
}
