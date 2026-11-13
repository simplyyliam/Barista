//! Built-in system services (ADR 0016): clock settings, battery, volume, notifications. Every service has the
//! same shape: a full snapshot you can read any time, a change event sent at most once per frame,
//! and (volume only) commands that change the system. No polling: Windows tells us when
//! something changed.
//!
//! ```text
//!  Windows event ─► service re-reads the snapshot ─► Service::set (only a real change wakes up)
//!                                                       └► forward task: wait 16 ms, emit latest
//! ```

pub mod actions;
pub mod app_volumes;
pub mod brightness;
pub mod hotspot;
pub mod media;
pub mod night_light;
pub mod user;
pub mod output_devices;
pub mod airplane;
pub mod battery;
pub mod bluetooth;
pub mod clock;
pub mod notifications;
pub mod theme;
pub mod volume;
pub mod wifi;
pub mod wifi_profile;

use std::time::Duration;
use tokio::sync::watch;

/// One frame. A burst of changes (a volume slider) becomes one event per frame, like `items-changed`.
const FRAME: Duration = Duration::from_millis(16);

/// Holds the latest snapshot of a service. Cheap to share between threads.
pub struct Service<T> {
    tx: watch::Sender<T>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Service<T> {
    pub fn new(initial: T) -> Self {
        Self {
            tx: watch::channel(initial).0,
        }
    }

    /// The current full snapshot.
    pub fn get(&self) -> T {
        self.tx.borrow().clone()
    }

    /// Stores a new snapshot. Nothing happens if it equals the old one.
    pub fn set(&self, value: T) {
        self.tx.send_if_modified(|old| {
            if *old == value {
                false
            } else {
                *old = value;
                true
            }
        });
    }

    pub fn subscribe(&self) -> watch::Receiver<T> {
        self.tx.subscribe()
    }

    /// Calls `emit` with the latest snapshot after each change, at most once per frame. The task
    /// sleeps while nothing changes.
    pub fn forward(&'static self, emit: impl Fn(T) + Send + 'static) {
        let mut rx = self.subscribe();
        tauri::async_runtime::spawn(async move {
            while rx.changed().await.is_ok() {
                tokio::time::sleep(FRAME).await;
                let value = rx.borrow_and_update().clone();
                emit(value);
            }
        });
    }
}

/// False in `windows` mode (CLAUDE.md rule 11): the slice B services publish nothing and the bar opens
/// Windows' Quick Settings instead. Their threads stay parked in `recv()`, so they cost nothing.
static CONTROL_CENTER_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// Held while a service publishes and while the mode switches, so a snapshot that was ready just before
/// the switch to `windows` mode can't land after the services were emptied.
static GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// `Service::set`, but only while the control center is ours.
pub fn publish_gated<T: Clone + PartialEq + Send + Sync + 'static>(service: &Service<T>, value: T) {
    let _gate = GATE.lock().unwrap_or_else(|e| e.into_inner());
    if control_center_active() {
        service.set(value);
    }
}

pub fn control_center_active() -> bool {
    CONTROL_CENTER_ACTIVE.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn control_center_mode() -> crate::settings::Mode {
    crate::controller::get()
        .map(|c| c.settings().control_center_mode)
        .unwrap_or_default()
}

/// Switches the control center between ours and Windows'. Saved.
pub fn set_control_center_mode(mode: crate::settings::Mode) {
    use crate::settings::Mode;
    if let Some(c) = crate::controller::get() {
        c.update_settings(|s| s.control_center_mode = mode);
    }
    let gate = GATE.lock().unwrap_or_else(|e| e.into_inner());
    CONTROL_CENTER_ACTIVE.store(mode == Mode::Barista, std::sync::atomic::Ordering::Relaxed);
    if mode == Mode::Barista {
        drop(gate);
        refresh_control_center();
    } else {
        brightness::service().set(Default::default());
        wifi::service().set(Default::default());
        bluetooth::service().set(Default::default());
        airplane::service().set(Default::default());
        hotspot::service().set(Default::default());
        output_devices::service().set(Default::default());
        user::service().set(Default::default());
        media::service().set(Default::default());
        night_light::service().set(Default::default());
        app_volumes::service().set(Default::default());
    }
}

/// Everything in the control center that has no change event of its own gets read again.
fn refresh_control_center() {
    if !control_center_active() {
        return;
    }
    brightness::refresh();
    hotspot::refresh();
    user::refresh();
    airplane::refresh();
    battery::refresh();
    media::refresh();
    night_light::refresh();
    wifi::scan();
    output_devices::refresh();
    app_volumes::refresh();
    #[cfg(windows)]
    crate::platform::windows::radios::refresh();
}

/// A panel opened. Opening is an event, not a poll: the control center re-reads what Windows
/// gives no change event for (hotspot, external monitors, the user) and asks for a Wi-Fi scan.
pub fn panel_opened(name: &str) {
    if name == "control-center" {
        refresh_control_center();
    }
    // Same switch as the control center: in "windows" mode nothing is read and the sliders do nothing.
    if name == "volume" && control_center_active() {
        app_volumes::refresh();
    }
}

/// Starts the services and their event forwarding. Call once from `setup`.
pub fn init(app: &tauri::AppHandle) {
    CONTROL_CENTER_ACTIVE.store(
        control_center_mode() == crate::settings::Mode::Barista,
        std::sync::atomic::Ordering::Relaxed,
    );
    use crate::events::{BrightnessChanged, HotspotChanged, UserChanged, OutputDevicesChanged, AirplaneModeChanged, 
        BatteryChanged, BluetoothChanged, ClockSettingsChanged, NotificationsChanged, VolumeChanged, WifiChanged,
    };
    use tauri_specta::Event;
    let a = app.clone();
    clock::service().forward(move |v| {
        let _ = ClockSettingsChanged(v).emit(&a);
    });
    let a = app.clone();
    battery::service().forward(move |v| {
        let _ = BatteryChanged(v).emit(&a);
    });
    let a = app.clone();
    volume::service().forward(move |v| {
        let _ = VolumeChanged(v).emit(&a);
    });
    let a = app.clone();
    notifications::service().forward(move |v| {
        let _ = NotificationsChanged(v).emit(&a);
    });
    let a = app.clone();
    bluetooth::service().forward(move |v| {
        let _ = BluetoothChanged(v).emit(&a);
    });
    let a = app.clone();
    wifi::service().forward(move |v| {
        let _ = WifiChanged(v).emit(&a);
    });
    let a = app.clone();
    airplane::service().forward(move |v| {
        let _ = AirplaneModeChanged(v).emit(&a);
    });
    let a = app.clone();
    output_devices::service().forward(move |v| {
        let _ = OutputDevicesChanged(v).emit(&a);
    });
    let a = app.clone();
    app_volumes::service().forward(move |v| {
        let _ = crate::events::AppVolumesChanged(v).emit(&a);
    });
    let a = app.clone();
    user::service().forward(move |v| {
        let _ = UserChanged(v).emit(&a);
    });
    let a = app.clone();
    hotspot::service().forward(move |v| {
        let _ = HotspotChanged(v).emit(&a);
    });
    let a = app.clone();
    brightness::service().forward(move |v| {
        let _ = BrightnessChanged(v).emit(&a);
    });
    let a = app.clone();
    media::service().forward(move |v| {
        let _ = crate::events::MediaChanged(v).emit(&a);
    });
    let a = app.clone();
    night_light::service().forward(move |v| {
        let _ = crate::events::NightLightChanged(v).emit(&a);
    });
    let a = app.clone();
    theme::service().forward(move |v| {
        let _ = crate::events::ThemeModeChanged(v).emit(&a);
    });
    clock::refresh();
    battery::refresh();
    battery::start_saver_watch();
    volume::start();
    notifications::start();
    #[cfg(windows)]
    crate::platform::windows::radios::start(radios_changed);
    bluetooth::start();
    wifi::start();
    brightness::start();
    hotspot::start();
    user::refresh();
    airplane::start();
    theme::start();
    media::start();
    night_light::start();
}

/// One radios thread serves Wi-Fi and Bluetooth: fan its state out to both services.
#[cfg(windows)]
fn radios_changed(r: crate::platform::windows::radios::RadioStates) {
    bluetooth::radio_changed(r);
    wifi::radio_changed(r);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_value_does_not_wake_anyone() {
        let s = Service::new(1);
        let rx = s.subscribe();
        s.set(1);
        assert!(!rx.has_changed().unwrap());
        s.set(2);
        assert!(rx.has_changed().unwrap());
        assert_eq!(s.get(), 2);
    }
}
