//! Built-in system services (ADR 0016): clock settings, battery, volume. Every service has the
//! same shape: a full snapshot you can read any time, a change event sent at most once per frame,
//! and (volume only) commands that change the system. No polling: Windows tells us when
//! something changed.
//!
//! ```text
//!  Windows event ─► service re-reads the snapshot ─► Service::set (only a real change wakes up)
//!                                                       └► forward task: wait 16 ms, emit latest
//! ```

pub mod battery;
pub mod clock;
pub mod volume;

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

/// Starts the three services and their event forwarding. Call once from `setup`.
pub fn init(app: &tauri::AppHandle) {
    use crate::events::{BatteryChanged, ClockSettingsChanged, VolumeChanged};
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
    clock::refresh();
    battery::refresh();
    volume::start();
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
