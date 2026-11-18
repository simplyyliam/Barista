//! Tray bridging: reads the real Windows tray icons and forwards clicks and menus, so they live
//! in the bar. Behind this module so the approach can be swapped (ADR 0012, built in ADR 0026).
//! v0.6 reads (this file), v0.7 makes them live (clicks and menus).
//!
//! ```text
//! platform/windows/tray_host.rs   the hidden Shell_TrayWnd: sees each message, forwards it to Explorer
//!        │ HostEvent (bounded channel, never waited on)
//!        ▼
//! worker (here)  parse.rs → state.rs (limits, add/modify/delete) → icons.rs (PNG cache) → registry
//! ```

pub mod click;
pub mod icons;
pub mod parse;
pub mod state;

#[cfg(windows)]
pub use worker::{click, init, set_mode, shutdown};

#[cfg(windows)]
mod worker {
    use super::click as sequences;
    use super::icons::{self, IconCache};
    use super::parse::{Action, TrayMessage};
    use super::state::{Outcome, Picture, TrayState};
    use crate::platform::windows::app_icon;
    use crate::platform::windows::tray_host::{self, Host, HostEvent, OwnedIcon};
    use crate::registry::{ClickError, Registry, TrayItem, TrayOwner};
    use barista_protocol::{MouseButton, ScreenRect};
    use crate::platform::windows::{foreground, tray_input};
    use crate::settings::Mode;
    use std::sync::mpsc::{sync_channel, Receiver};
    use std::sync::{Arc, Mutex};

    /// Events the host may have waiting for the worker. Past this it drops them (counted).
    const QUEUE: usize = 1024;

    static HOST: Mutex<Option<Host>> = Mutex::new(None);
    /// Kept so `set_mode` can start the host again (ADR 0032).
    static REGISTRY: std::sync::OnceLock<Arc<Registry>> = std::sync::OnceLock::new();

    /// Switches the tray between ours and Explorer's while the bar runs. `windows` closes the host
    /// window (Explorer gets every message again) and empties the bar's tray icons; `barista` starts
    /// the host, which announces `TaskbarCreated` so running apps add their icons back.
    pub fn set_mode(mode: Mode) {
        let Some(registry) = REGISTRY.get().cloned() else { return };
        let running = HOST.lock().unwrap_or_else(|e| e.into_inner()).is_some();
        match mode {
            Mode::Windows if running => {
                shutdown();
                registry.tray_clear();
            }
            Mode::Windows => registry.tray_clear(),
            Mode::Barista if !running => init(registry),
            Mode::Barista => {}
        }
    }

    /// Starts the tray host and the worker, unless the user picked Windows' own tray or another
    /// program already hosts it. Call once, after the controller (it reads the settings).
    pub fn init(registry: Arc<Registry>) {
        let _ = REGISTRY.set(registry.clone());
        let mode = crate::controller::get()
            .map(|c| c.settings().advanced.tray)
            .unwrap_or_default();
        if mode == Mode::Windows {
            eprintln!("[tray] trayMode is windows: the tray stays with Explorer");
            return;
        }
        if let Some(name) = tray_host::foreign_host() {
            eprintln!("[tray] {name} already hosts the tray window or edits the taskbar: not starting ours");
            return;
        }
        let (tx, rx) = sync_channel(QUEUE);
        match tray_host::start(tx) {
            Ok(host) => {
                *HOST.lock().unwrap_or_else(|e| e.into_inner()) = Some(host);
                retry_announce(registry.clone());
                let spawned = std::thread::Builder::new()
                    .name("tray-worker".into())
                    .spawn(move || Worker::new(registry).run(rx));
                if let Err(e) = spawned {
                    eprintln!("[tray] could not start the worker: {e}");
                }
            }
            Err(e) => eprintln!("[tray] the tray host did not start: {e}"),
        }
    }

    /// Seen once: after a start no tray app re-added its icon (the host worked, a manual broadcast
    /// brought them all back). So if the tray is still empty a few seconds after the first broadcast,
    /// broadcast again, at most twice. Two sleeps, not a poll; a machine with no tray apps just gets
    /// two harmless broadcasts.
    fn retry_announce(registry: Arc<Registry>) {
        let _ = std::thread::Builder::new().name("tray-announce".into()).spawn(move || {
            for wait in [4u64, 8] {
                std::thread::sleep(std::time::Duration::from_secs(wait));
                if registry.tray_len() > 0 {
                    return;
                }
                eprintln!("[tray] no icon arrived {wait} s after start: announcing the taskbar again");
                tray_host::announce();
            }
        });
    }

    /// Closes the host window, so messages reach Explorer alone again (on a normal quit).
    pub fn shutdown() {
        let stats = tray_host::stats();
        eprintln!(
            "[tray] quit: {} tray messages, {} times raised above Explorer, {} dropped, {} not answered by Explorer",
            stats.tray_messages, stats.reraised, stats.dropped, stats.forward_failed
        );
        drop(HOST.lock().unwrap_or_else(|e| e.into_inner()).take());
    }

    static DOUBLE: Mutex<sequences::DoubleClick> = Mutex::new(sequences::DoubleClick::default_const());

    /// A click on the tray icon `key` (`tray/<key>`): posts the run of mouse messages the real tray
    /// would, to the app's own window (ADR 0027). `rect` is the icon in the bar, in screen pixels,
    /// when the UI told us (the anchor); without it the mouse pointer stands in.
    pub fn click(
        registry: &Registry,
        key: &str,
        button: MouseButton,
        rect: Option<ScreenRect>,
    ) -> Result<(), ClickError> {
        let (owner, pid) = registry.tray_owner(key).ok_or(ClickError::NotFound)?;
        let callback = owner.callback_message.ok_or(ClickError::NotFound)?;
        // A dead window: the icon is about to be removed by the destroy hook, say so.
        if !tray_host::window_exists(owner.hwnd) {
            return Err(ClickError::NotFound);
        }
        let double = {
            let mut tracker = DOUBLE.lock().unwrap_or_else(|e| e.into_inner());
            if button == MouseButton::Left {
                tracker.left(key, now_ms(), tray_input::double_click_ms())
            } else {
                tracker.reset();
                false
            }
        };
        // Bottom centre of the icon, where a menu would drop from; else the pointer.
        let anchor = rect
            .map(|r| (r.x + r.width / 2, r.y + r.height))
            .or_else(tray_input::cursor)
            .unwrap_or((0, 0));
        // The app must be allowed to take focus, or its menu would not close on an outside click.
        foreground::allow(pid);
        let mut delivered = true;
        for message in sequences::sequence(button, owner.version, double) {
            let (w, l) = sequences::encode(message, owner.version, owner.id, anchor);
            delivered &= tray_input::post(owner.hwnd, callback, w, l);
        }
        if delivered {
            Ok(())
        } else {
            // The app never saw this click, so the next one must not count as its double.
            DOUBLE.lock().unwrap_or_else(|e| e.into_inner()).reset();
            Err(ClickError::Unreachable)
        }
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    }

    struct Worker {
        registry: Arc<Registry>,
        state: TrayState,
        cache: IconCache,
        /// How many events the host had dropped when we last looked (queue full).
        dropped_seen: u64,
    }

    impl Worker {
        fn new(registry: Arc<Registry>) -> Self {
            Self {
                registry,
                state: TrayState::default(),
                cache: IconCache::default(),
                dropped_seen: 0,
            }
        }

        fn run(mut self, events: Receiver<HostEvent>) {
            // Sleeps in `recv` until the host sends something: no polling.
            while let Ok(event) = events.recv() {
                self.handle(event);
                self.reconcile_if_events_were_lost();
            }
        }

        fn handle(&mut self, event: HostEvent) {
            match event {
                HostEvent::Tray { message, icon, pid } => {
                    // pid 0: the owner's window is already gone, so there is nobody to show.
                    if pid == 0 && message.action == Action::Add {
                        return;
                    }
                    let picture = self.picture(&message, icon);
                    let name = if message.action == Action::Add {
                        tray_host::process_name(pid).unwrap_or_else(|| "Unknown".into())
                    } else {
                        String::new()
                    };
                    let outcome = self.state.apply(&message, pid, &name, picture);
                    self.publish(outcome);
                }
                HostEvent::WindowGone(hwnd) => {
                    let outcome = self.state.remove_window(hwnd);
                    self.publish(outcome);
                    tray_host::unwatch(hwnd);
                }
            }
        }

        /// The host drops events when we fall behind. A lost "window gone" or delete would leave a
        /// ghost icon for good, so after a drop we check every owner window once and remove the
        /// ones that are gone. (A lost add is only healed when the app adds its icon again.)
        fn reconcile_if_events_were_lost(&mut self) {
            let dropped = tray_host::stats().dropped;
            if dropped == self.dropped_seen {
                return;
            }
            self.dropped_seen = dropped;
            for hwnd in self.state.windows() {
                if !tray_host::window_exists(hwnd) {
                    let outcome = self.state.remove_window(hwnd);
                    self.publish(outcome);
                    tray_host::unwatch(hwnd);
                }
            }
        }

        /// What the message does to the picture (converting the icon copy if it has one).
        fn picture(&mut self, message: &TrayMessage, icon: Option<OwnedIcon>) -> Picture {
            match (message.icon_handle, icon) {
                (None, _) => Picture::Keep,
                (Some(0), _) => Picture::Clear,
                // The host could not copy the icon (the app already freed it): keep what we have.
                (Some(_), None) => Picture::Keep,
                (Some(_), Some(icon)) => self.convert(&icon),
            }
        }

        fn convert(&mut self, icon: &OwnedIcon) -> Picture {
            let handle = icon.handle();
            let convert = || {
                app_icon::icon_png_from_hicon(handle).and_then(|png| icons::png_to_data_url(&png))
            };
            let url = match app_icon::icon_fingerprint(handle) {
                Some(fingerprint) => self.cache.get_or_convert(fingerprint, convert),
                None => convert(),
            };
            // A failed conversion keeps the picture we have: it must not blank a working icon.
            url.map_or(Picture::Keep, Picture::Set)
        }

        fn publish(&self, outcome: Outcome) {
            match outcome {
                Outcome::Upsert(key) => {
                    if let Some(icon) = self.state.get(&key) {
                        self.registry.tray_set(
                            &key.text(),
                            TrayItem {
                                app_name: icon.app_name.clone(),
                                pid: icon.pid,
                                stable_id: super::state::stable_id(&key, &icon.app_name, icon.id),
                                owner: TrayOwner {
                                    hwnd: icon.hwnd,
                                    id: icon.id,
                                    callback_message: icon.callback_message,
                                    version: icon.version,
                                },
                                icon: icon.icon.clone(),
                                tooltip: icon.tip.clone(),
                                visible: !icon.hidden,
                            },
                        );
                    }
                }
                Outcome::Remove(keys) => {
                    for key in keys {
                        self.registry.tray_remove(&key.text());
                    }
                }
                Outcome::Nothing => {}
            }
        }
    }
}
