//! The bar's adaptive foreground: is the wallpaper behind the bar light or dark? (ADR 0009)
//!
//! One worker thread does the measuring. It sleeps until someone sends it a message, waits for
//! changes to settle (150 ms), measures once, and emits `bar-appearance-changed` only when the
//! answer really changed. Nothing polls (CLAUDE.md rule 9).
//!
//! Messages come from `controller.rs` (the bar moved to another monitor, DPI or display change)
//! and from the window hook (the wallpaper changed).

pub mod layout;
pub mod luminance;

use crate::bar::Rect;
use luminance::{tone_for, Tone};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::AppHandle;
use tauri_specta::Event;

/// What the UI needs to know. `tone` is the tone of the WALLPAPER: `light` means dark content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct BarAppearance {
    pub tone: Tone,
}

/// The wallpaper behind the bar changed from light to dark or back (or the debug override did).
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct BarAppearanceChanged(pub BarAppearance);

enum Msg {
    /// The bar sits on this monitor rectangle and is this many physical pixels tall.
    Layout(Rect, i32),
    WallpaperChanged,
}

#[derive(Clone, Copy)]
struct State {
    measured: Tone,
    /// Debug builds only: a tone forced from the keyboard.
    forced: Option<Tone>,
}

impl State {
    fn effective(&self) -> Tone {
        self.forced.unwrap_or(self.measured)
    }
}

static STATE: Mutex<State> = Mutex::new(State {
    // Light is also the page's default, so nothing flickers before the first measurement.
    measured: Tone::Light,
    forced: None,
});
static SENDER: OnceLock<Sender<Msg>> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();

const SETTLE: Duration = Duration::from_millis(150);

fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// The tone right now (for the `get_bar_appearance` command).
pub fn current() -> BarAppearance {
    BarAppearance {
        tone: lock().effective(),
    }
}

/// Starts the worker. Call once at startup.
pub fn start(app: &AppHandle) {
    if APP.set(app.clone()).is_err() {
        return;
    }
    let (tx, rx) = channel::<Msg>();
    let _ = SENDER.set(tx);
    let spawned = std::thread::Builder::new()
        .name("appearance".into())
        .spawn(move || {
            #[cfg(windows)]
            crate::platform::windows::wallpaper::init_com();
            let mut layout: Option<(Rect, i32)> = None;
            // Sleeps here while nothing happens: zero CPU.
            while let Ok(first) = rx.recv() {
                let mut msg = Some(first);
                // Let a burst of messages (a display change sends several) settle, keep the latest layout.
                while let Some(m) = msg.take() {
                    if let Msg::Layout(r, h) = m {
                        layout = Some((r, h));
                    }
                    match rx.recv_timeout(SETTLE) {
                        Ok(next) => msg = Some(next),
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                if let Some((rect, strip_h)) = layout {
                    measure(rect, strip_h);
                }
            }
        });
    if let Err(e) = spawned {
        eprintln!("barista: could not start the appearance worker: {e}");
    }
}

/// Tells the worker where the bar is. Cheap: it only sends a message.
pub fn request(monitor: Rect, strip_h: i32) {
    if let Some(tx) = SENDER.get() {
        let _ = tx.send(Msg::Layout(monitor, strip_h));
    }
}

/// Tells the worker the wallpaper changed.
pub fn wallpaper_changed() {
    if let Some(tx) = SENDER.get() {
        let _ = tx.send(Msg::WallpaperChanged);
    }
}

fn measure(monitor: Rect, strip_h: i32) {
    #[cfg(windows)]
    {
        let started = std::time::Instant::now();
        let Some(l) = crate::platform::windows::wallpaper::strip_luminance(monitor, strip_h) else {
            // Could not read it: keep what we had.
            return;
        };
        let tone = tone_for(l);
        if std::env::var_os("BARISTA_DEBUG").is_some() {
            eprintln!(
                "barista: wallpaper strip luminance {l:.3} -> {tone:?} in {:?}",
                started.elapsed()
            );
        }
        set_measured(tone);
    }
    #[cfg(not(windows))]
    let _ = (monitor, strip_h);
}

fn set_measured(tone: Tone) {
    let (before, after) = {
        let mut s = lock();
        let before = s.effective();
        s.measured = tone;
        (before, s.effective())
    };
    if before != after {
        emit(after);
    }
}

/// Debug builds: cycles auto -> light -> dark -> auto and emits the normal event.
#[cfg(debug_assertions)]
pub fn cycle_override() {
    let (before, after) = {
        let mut s = lock();
        let before = s.effective();
        s.forced = match s.forced {
            None => Some(Tone::Light),
            Some(Tone::Light) => Some(Tone::Dark),
            Some(Tone::Dark) => None,
        };
        (before, s.effective())
    };
    eprintln!("barista: tone override -> {after:?}");
    if before != after {
        emit(after);
    }
}

fn emit(tone: Tone) {
    if let Some(app) = APP.get() {
        let _ = BarAppearanceChanged(BarAppearance { tone }).emit(app);
    }
}
