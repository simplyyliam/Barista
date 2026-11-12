//! Puts the laptop panel (WMI) and the external monitors (DDC/CI) behind one worker thread.
//!
//! DDC can hang (a dock, a monitor that never answers), so it has its own thread: the laptop panel
//! and the brightness keys (WMI) keep working while a monitor is slow. A burst of sets (a slider)
//! keeps only the last value per display. No polling: the WMI event thread and the app's own events
//! wake us.

use super::{ddc, wmi_brightness};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
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

/// For the WMI thread (the laptop panel).
enum Msg {
    RefreshAll,
    RefreshInternal,
    /// The laptop panel, 0 to 100.
    Set(u8),
}

/// For the DDC thread (external monitors, which can be very slow).
enum DdcMsg {
    /// Read every monitor except the one with this PnP id (the laptop panel).
    Refresh(Option<String>),
    Set(String, u8),
}

static TX: OnceLock<Sender<Msg>> = OnceLock::new();
static DDC_TX: OnceLock<Sender<DdcMsg>> = OnceLock::new();

/// What both threads have found so far.
#[derive(Default)]
struct Shared {
    internal: Option<(u8, String)>,
    external: Vec<ddc::RawDdc>,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared {
    internal: None,
    external: Vec::new(),
});

pub fn refresh_all() {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::RefreshAll);
    }
}

pub fn set(id: String, percent: u8) {
    if id == INTERNAL_ID {
        if let Some(tx) = TX.get() {
            let _ = tx.send(Msg::Set(percent));
        }
    } else if let Some(tx) = DDC_TX.get() {
        let _ = tx.send(DdcMsg::Set(id, percent));
    }
}

/// Builds the list from what is known and hands it on.
fn publish(on_change: fn(Vec<RawDisplay>)) {
    let mut list = Vec::new();
    {
        let shared = SHARED.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((percent, _)) = &shared.internal {
            list.push(RawDisplay {
                id: INTERNAL_ID.into(),
                name: "Built-in display".into(),
                internal: true,
                percent: Some(*percent),
            });
        }
        list.extend(shared.external.iter().map(|d| RawDisplay {
            id: d.id.clone(),
            name: d.name.clone(),
            internal: false,
            percent: d.percent,
        }));
    }
    on_change(list);
}

fn init_com() {
    // SAFETY: plain COM init for the calling thread; WMI and the monitor API need it.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
    }
}

/// External monitors: the only thread that may block for a long time.
fn ddc_thread(rx: std::sync::mpsc::Receiver<DdcMsg>, on_change: fn(Vec<RawDisplay>)) {
    init_com();
    while let Ok(first) = rx.recv() {
        let mut sets: Vec<(String, u8)> = Vec::new();
        let mut refresh: Option<Option<String>> = None;
        for m in std::iter::once(first).chain(std::iter::from_fn(|| rx.try_recv().ok())) {
            match m {
                DdcMsg::Refresh(skip) => refresh = Some(skip),
                DdcMsg::Set(id, p) => match sets.iter_mut().find(|(i, _)| *i == id) {
                    Some(s) => s.1 = p,
                    None => sets.push((id, p)),
                },
            }
        }
        for (id, percent) in sets {
            if ddc::set(&id, percent) {
                // Trust the monitor: reading back over the cable is slow.
                let mut shared = SHARED.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(d) = shared.external.iter_mut().find(|d| d.id == id) {
                    d.percent = Some(percent);
                }
            }
        }
        publish(on_change);
        if let Some(skip) = refresh {
            // No lock while the monitors answer.
            let read = ddc::read_all(skip.as_deref());
            SHARED.lock().unwrap_or_else(|e| e.into_inner()).external = read;
            publish(on_change);
        }
    }
}

pub fn start(on_change: fn(Vec<RawDisplay>)) {
    let (tx, rx) = channel();
    let (ddc_tx, ddc_rx) = channel();
    if TX.set(tx.clone()).is_err() || DDC_TX.set(ddc_tx.clone()).is_err() {
        return;
    }
    if let Err(e) = std::thread::Builder::new()
        .name("barista-ddc".into())
        .spawn(move || ddc_thread(ddc_rx, on_change))
    {
        eprintln!("barista: could not start the DDC thread: {e}");
    }
    let spawned = std::thread::Builder::new()
        .name("barista-brightness".into())
        .spawn(move || {
            init_com();
            let wmi = wmi_brightness::connect();
            // The brightness keys and Windows settings: blocks in WMI until one happens.
            let event_tx = tx.clone();
            let _ = std::thread::Builder::new()
                .name("barista-brightness-events".into())
                .spawn(move || {
                    init_com();
                    wmi_brightness::watch(|| {
                        let _ = event_tx.send(Msg::RefreshInternal);
                    });
                });

            let mut msg = Some(Msg::RefreshAll);
            while let Some(first) = msg.take().or_else(|| rx.recv().ok()) {
                // Take everything queued: a slider's burst becomes one pass, last value wins.
                let mut set: Option<u8> = None;
                let (mut all, mut inner) = (false, false);
                for m in std::iter::once(first).chain(std::iter::from_fn(|| rx.try_recv().ok())) {
                    match m {
                        Msg::RefreshAll => all = true,
                        Msg::RefreshInternal => inner = true,
                        Msg::Set(p) => set = Some(p),
                    }
                }
                if let (Some(percent), Some(w)) = (set, &wmi) {
                    if w.set(percent) {
                        if let Some((p, _)) = SHARED.lock().unwrap_or_else(|e| e.into_inner()).internal.as_mut() {
                            *p = percent;
                        }
                    }
                }
                if all || inner {
                    let read = wmi.as_ref().and_then(|w| w.read_with_id());
                    SHARED.lock().unwrap_or_else(|e| e.into_inner()).internal = read;
                }
                publish(on_change);
                if all {
                    let skip = SHARED
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .internal
                        .as_ref()
                        .map(|(_, pnp)| pnp.clone());
                    let _ = ddc_tx.send(DdcMsg::Refresh(skip));
                }
            }
        });
    if let Err(e) = spawned {
        eprintln!("barista: could not start the brightness thread: {e}");
    }
}
