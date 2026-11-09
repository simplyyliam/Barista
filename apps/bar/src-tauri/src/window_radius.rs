//! Keeps each window's frontend up to date on its corner radius.
//!
//! Every window that wants a radius is registered by its label ([`register`]) and gets its own
//! tracker, so the settings window can be square while maximized and a popover stays rounded.
//! Two things can change a radius: the window changes state (maximize, snap, full screen, ...),
//! or the Windhawk mod's settings change in the registry. Both end in a recompute that sends
//! an update to *that window only*, and only when its value really changed.
//!
//! A window gets its radius three ways:
//! - `commands.getWindowRadius()`: the value for the window that asks (first value on start),
//! - the typed event `windowRadiusChanged`, delivered to that window and carrying its `label`,
//! - the CSS variable `--window-radius` on that window's own `<html>`, so a stylesheet can say
//!   `border-radius: var(--window-radius, 8px)`.
//!
//! Child radii (content inside a rounded window, with padding) need no Rust: the nested radius is
//! the parent's minus the padding, which CSS can do with
//! `border-radius: max(0px, calc(var(--window-radius) - 12px))`.

use crate::events::WindowRadiusChanged;
use crate::platform::windows::{windhawk, window};
use crate::radius::{radius_for, ModSettings, RadiusTracker, WindowRadius, WindowState};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tauri::{AppHandle, Manager, WindowEvent};
use tauri_specta::Event;
use windows::Win32::Foundation::HWND;

/// How a window's state is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Look at the real window: maximized, snapped and full screen windows get the square (or
    /// the mod's) corners of that state. For windows the user can maximize or snap, like the
    /// settings window.
    Tracked,
    /// Always a normal window: the radius is the mod's radius, or 8 without it, with no state
    /// detection at all. For windows that are never maximized or snapped, like a transparent
    /// popover.
    // Nothing registers a window in this mode before popovers exist (v0.4).
    #[allow(dead_code)]
    Normal,
}

/// The state to compute a window's radius for. `Normal` mode never calls `detect`, so such a
/// window costs no Win32 queries at all.
fn state_for(mode: Mode, detect: impl FnOnce() -> Option<WindowState>) -> Option<WindowState> {
    match mode {
        Mode::Normal => Some(WindowState::Normal),
        Mode::Tracked => detect(),
    }
}

struct Entry {
    mode: Mode,
    /// The native window handle (as an integer, so the map stays `Send`). Only used in
    /// `Tracked` mode.
    hwnd: isize,
    tracker: RadiusTracker,
}

#[derive(Default)]
struct Inner {
    /// What the registry said last; `None` when the mod is not active. Shared by all windows.
    settings: Option<ModSettings>,
    windows: HashMap<String, Entry>,
}

struct State {
    app: AppHandle,
    inner: Mutex<Inner>,
    /// Keeps the registry watcher thread alive for the whole run.
    watcher: Mutex<Option<windhawk::Watcher>>,
}

static STATE: OnceLock<State> = OnceLock::new();

fn lock(state: &State) -> MutexGuard<'_, Inner> {
    // A panic while the lock was held leaves plain data behind, so ignore the poison flag.
    state.inner.lock().unwrap_or_else(|e| e.into_inner())
}

/// Starts giving the window `label` a radius. Call once per window, after it exists.
/// Registering the first window also starts watching the Windhawk settings.
pub fn register(app: &AppHandle, label: &str, mode: Mode) -> Result<(), String> {
    let webview = app
        .get_webview_window(label)
        .ok_or_else(|| format!("no window {label:?}"))?;
    let hwnd = webview.hwnd().map_err(|e| e.to_string())?.0 as isize;

    let state = STATE.get_or_init(|| State {
        app: app.clone(),
        inner: Mutex::new(Inner::default()),
        watcher: Mutex::new(None),
    });
    lock(state).windows.insert(
        label.to_string(),
        Entry {
            mode,
            hwnd,
            tracker: RadiusTracker::default(),
        },
    );

    if mode == Mode::Tracked {
        // Windows reports a resize or move for every maximize, restore, snap and full-screen
        // change, so these two events are enough to notice a state change.
        let owned = label.to_string();
        webview.on_window_event(move |event| match event {
            WindowEvent::Resized(_)
            | WindowEvent::Moved(_)
            | WindowEvent::ScaleFactorChanged { .. } => recompute(&owned),
            WindowEvent::Destroyed => unregister(&owned),
            _ => {}
        });
    } else {
        let owned = label.to_string();
        webview.on_window_event(move |event| {
            if matches!(event, WindowEvent::Destroyed) {
                unregister(&owned);
            }
        });
    }

    start_watcher(state);
    // The watcher's first call reads the registry and recomputes every window, but when it was
    // already running for an earlier window this one still needs its first value.
    recompute(label);
    Ok(())
}

/// Forgets a window (it was destroyed).
pub fn unregister(label: &str) {
    if let Some(state) = STATE.get() {
        lock(state).windows.remove(label);
    }
}

fn start_watcher(state: &'static State) {
    let mut watcher = state.watcher.lock().unwrap_or_else(|e| e.into_inner());
    if watcher.is_some() {
        return;
    }
    // Read once now, so the first value a window gets is already the right one. (The watcher
    // reads again on its thread, which changes nothing and so sends nothing.)
    lock(state).settings = windhawk::read_settings();
    // The watcher calls us once right away (which does the first read) and then on every
    // change of the mod's keys. A failure to watch is not fatal: the radius then stays at the
    // value read now.
    match windhawk::watch(refresh_settings) {
        Ok(started) => *watcher = Some(started),
        Err(error) => {
            eprintln!("barista: cannot watch the Windhawk settings: {error}");
            drop(watcher);
            refresh_settings();
        }
    }
}

/// Reads the registry again, then recomputes every window. Runs on the watcher thread.
fn refresh_settings() {
    let Some(state) = STATE.get() else { return };
    let settings = windhawk::read_settings();
    let changed: Vec<String> = {
        let mut inner = lock(state);
        inner.settings = settings;
        let labels: Vec<String> = inner.windows.keys().cloned().collect();
        labels
            .into_iter()
            .filter(|label| update_locked(&mut inner, label))
            .collect()
    };
    for label in changed {
        announce(state, &label);
    }
}

/// Recomputes one window. Cheap enough to call on every resize or move.
pub fn recompute(label: &str) {
    let Some(state) = STATE.get() else { return };
    let changed = update_locked(&mut lock(state), label);
    if changed {
        announce(state, label);
    }
}

/// Works out the window's radius and stores it. True when it differs from what was stored.
/// Only touches our own data: the lock is never held while talking to the webview.
fn update_locked(inner: &mut Inner, label: &str) -> bool {
    let settings = inner.settings;
    let Some(entry) = inner.windows.get_mut(label) else {
        return false;
    };
    // The window may be gone: keep the last value then.
    let Some(window_state) = state_for(entry.mode, || {
        window::window_state(HWND(entry.hwnd as *mut _))
    }) else {
        return false;
    };
    entry
        .tracker
        .update(radius_for(window_state, settings.as_ref()))
        .is_some()
}

/// Tells the window about its radius: the typed event (to that window only, with its label in
/// the payload) and the CSS variable. Sending happens outside the lock because it talks to the
/// webview. If two threads race here, both send the *latest* value, so the window always ends
/// on the right one.
fn announce(state: &State, label: &str) {
    let value = lock(state)
        .windows
        .get(label)
        .map(|entry| entry.tracker.current());
    let Some(value) = value else { return }; // unregistered meanwhile
    let payload = WindowRadiusChanged {
        label: label.to_string(),
        radius: value,
    };
    let _ = payload.emit_to(&state.app, label);
    apply_css(&state.app, label, value);
}

/// The value for the window `label` (`radius: None` before its first value is known).
pub fn current(label: &str) -> WindowRadius {
    let unknown = RadiusTracker::default().current();
    let Some(state) = STATE.get() else {
        return unknown;
    };
    lock(state)
        .windows
        .get(label)
        .map_or(unknown, |entry| entry.tracker.current())
}

/// Puts the window's current value on its `<html>` again. A page reload throws the variable
/// away, so this runs after every page load.
pub fn reapply_css(app: &AppHandle, label: &str) {
    apply_css(app, label, current(label));
}

fn apply_css(app: &AppHandle, label: &str, value: WindowRadius) {
    let Some(webview) = app.get_webview_window(label) else {
        return;
    };
    // `radius` is a number, so nothing here can break out of the string.
    let script = match value.radius {
        Some(radius) => {
            format!("document.documentElement.style.setProperty('--window-radius', '{radius}px')")
        }
        None => "document.documentElement.style.removeProperty('--window-radius')".to_string(),
    };
    // Before the page exists this fails; the page-load hook applies it again.
    let _ = webview.eval(script);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_mode_never_looks_at_the_window() {
        let state = state_for(Mode::Normal, || panic!("must not query the window"));
        assert_eq!(state, Some(WindowState::Normal));
    }

    #[test]
    fn tracked_mode_uses_what_the_window_reports() {
        assert_eq!(
            state_for(Mode::Tracked, || Some(WindowState::Maximized)),
            Some(WindowState::Maximized)
        );
        assert_eq!(state_for(Mode::Tracked, || None), None);
    }
}
