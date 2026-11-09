//! Runs the bar window: owns the state, reacts to screen changes, and is the one place that
//! calls `platform::windows` and emits `bar-changed`.

use crate::bar::{bar_rect, pick_monitor, BarInfo, Monitor, Rect};
use crate::events::BarChanged;
use crate::platform::windows::{appbar, fullscreen, monitors, window};
use crate::settings::{settings_path, Settings};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;
use windows::Win32::Foundation::HWND;

struct State {
    settings: Settings,
    settings_file: PathBuf,
    monitors: Vec<Monitor>,
    current: Monitor,
    hidden_for_fullscreen: bool,
    /// The strip Windows granted last time, to skip work when nothing changed.
    last_rect: Option<Rect>,
    last_visible: bool,
    /// What the UI last heard, so `bar-changed` fires on any change (monitor list, scale, ...).
    last_info: Option<BarInfo>,
}

pub struct Controller {
    app: AppHandle,
    hwnd: isize,
    state: Mutex<State>,
    /// `apply` calls Win32, and Win32 can call back into our window procedure, which calls
    /// `apply` again on the same thread. These two flags turn that into one extra loop
    /// instead of a deadlock or endless recursion.
    applying: AtomicBool,
    again: AtomicBool,
}

static CONTROLLER: OnceLock<Controller> = OnceLock::new();

pub fn get() -> Option<&'static Controller> {
    CONTROLLER.get()
}

/// Sets the bar up. Must run on the main thread (Tauri's `setup` does).
pub fn start(app: &AppHandle) -> Result<(), String> {
    let win = app.get_webview_window("main").ok_or("no main window")?;
    let hwnd = win.hwnd().map_err(|e| e.to_string())?;

    let settings_file = settings_path(&app.path().app_config_dir().map_err(|e| e.to_string())?);
    let settings = Settings::load(&settings_file);
    let monitors = monitors::list();
    // No monitor at launch (display reset, session switch): start on a placeholder instead of
    // failing. Windows sends WM_DISPLAYCHANGE when a real one appears, and `refresh_monitors`
    // then replaces it.
    let current = pick_monitor(settings.monitor.as_deref(), &monitors)
        .cloned()
        .unwrap_or_else(placeholder_monitor);

    let controller = Controller {
        app: app.clone(),
        hwnd: hwnd.0 as isize,
        state: Mutex::new(State {
            settings,
            settings_file,
            monitors,
            current,
            hidden_for_fullscreen: false,
            last_rect: None,
            last_visible: false,
            last_info: None,
        }),
        applying: AtomicBool::new(false),
        again: AtomicBool::new(false),
    };
    CONTROLLER.set(controller).map_err(|_| "bar already started")?;
    let c = get().expect("just set");

    window::set_no_focus_styles(hwnd);
    if !appbar::register(hwnd) {
        eprintln!("barista: Windows refused the appbar registration; no space will be reserved");
    }
    window::install(c.hwnd, on_window_event);
    fullscreen::watch_foreground(on_foreground_changed);
    install_panic_hook();

    c.apply();
    Ok(())
}

fn placeholder_monitor() -> Monitor {
    let rect = Rect { left: 0, top: 0, right: 1920, bottom: 1080 };
    Monitor {
        id: String::new(),
        name: "no monitor".into(),
        is_primary: true,
        rect,
        work_area: rect,
        scale: 1.0,
    }
}

/// If the bar crashes, give the screen space back before the process dies.
/// (A kill from Task Manager runs no code at all: see ADR 0002.)
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        appbar::unregister();
        previous(info);
    }));
}

/// Runs `f`; if it panics, the panic hook has already released the screen space, so reserve
/// it again and carry on (the bar must survive one bad event).
fn guarded(f: impl FnOnce(&'static Controller)) {
    let Some(c) = get() else { return };
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(c))).is_err() {
        appbar::forget();
        appbar::register(c.hwnd());
        c.lock().last_rect = None;
        c.apply();
    }
}

fn on_window_event(event: window::WindowEvent) {
    guarded(|c| handle_window_event(c, event));
}

fn handle_window_event(c: &'static Controller, event: window::WindowEvent) {
    use window::WindowEvent::*;
    match event {
        PosChanged => c.apply(),
        // The shell's signal is global (a game on monitor 2 also reaches us), so only hide when
        // the window in front really covers the bar's own monitor.
        FullscreenApp(on) => {
            let monitor = c.lock().current.rect;
            c.set_fullscreen(on && fullscreen::foreground_covers(&monitor, c.hwnd()));
        }
        DisplayChanged | DpiChanged => {
            c.refresh_monitors();
            c.apply();
        }
        TaskbarCreated => {
            appbar::forget();
            appbar::register(c.hwnd());
            c.last_rect_reset();
            c.apply();
        }
    }
}

fn on_foreground_changed() {
    guarded(|c| {
        let monitor = c.lock().current.rect;
        c.set_fullscreen(fullscreen::foreground_covers(&monitor, c.hwnd()));
    });
}

impl Controller {
    fn hwnd(&self) -> HWND {
        HWND(self.hwnd as *mut _)
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // A poisoned lock only means another thread panicked; the data is still usable.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn last_rect_reset(&self) {
        self.lock().last_rect = None;
    }

    fn info_of(state: &State) -> BarInfo {
        let height_px = bar_rect(&state.current.rect, state.current.scale, state.settings.bar_height)
            .height();
        BarInfo {
            height: state.settings.bar_height,
            height_px,
            scale: state.current.scale,
            monitor: state.current.clone(),
            monitors: state.monitors.clone(),
            hidden_for_fullscreen: state.hidden_for_fullscreen,
        }
    }

    pub fn info(&self) -> BarInfo {
        Self::info_of(&self.lock())
    }

    /// Re-reads the monitors and picks where the bar goes: the saved one if plugged in,
    /// else the primary. The saved id stays, so the bar returns when the monitor does.
    fn refresh_monitors(&self) {
        let mut state = self.lock();
        let list = monitors::list();
        if let Some(m) = pick_monitor(state.settings.monitor.as_deref(), &list) {
            state.current = m.clone();
            state.monitors = list;
        }
        // An empty list (mid-unplug) keeps the old values until Windows sends the next change.
    }

    pub fn set_monitor(&self, id: &str) -> Result<BarInfo, String> {
        self.refresh_monitors();
        {
            let mut state = self.lock();
            if !state.monitors.iter().any(|m| m.id == id) {
                return Err(format!("unknown monitor: {id}"));
            }
            state.settings.monitor = Some(id.to_string());
            if let Err(e) = state.settings.save(&state.settings_file) {
                eprintln!("barista: could not save settings: {e}");
            }
            if let Some(m) = pick_monitor(Some(id), &state.monitors) {
                state.current = m.clone();
            }
        }
        // Move the window on the UI thread: this command may be running on another one.
        let app = self.app.clone();
        let _ = app.run_on_main_thread(|| {
            if let Some(c) = get() {
                c.apply();
            }
        });
        Ok(self.info())
    }

    fn set_fullscreen(&self, on: bool) {
        {
            let mut state = self.lock();
            if state.hidden_for_fullscreen == on {
                return;
            }
            state.hidden_for_fullscreen = on;
        }
        self.apply();
    }

    /// Puts the window where it belongs: reserve the strip, move there, show or hide.
    fn apply(&self) {
        if self.applying.swap(true, Ordering::SeqCst) {
            self.again.store(true, Ordering::SeqCst);
            return;
        }
        // Clears the flags even if `apply_once` panics, so one bad round can't freeze the bar.
        struct Reset<'a>(&'a Controller);
        impl Drop for Reset<'_> {
            fn drop(&mut self) {
                self.0.again.store(false, Ordering::SeqCst);
                self.0.applying.store(false, Ordering::SeqCst);
            }
        }
        let _reset = Reset(self);
        // Bounded: a few extra rounds settle any ping-pong with other appbars.
        for _ in 0..4 {
            self.apply_once();
            if !self.again.swap(false, Ordering::SeqCst) {
                break;
            }
        }
    }

    fn apply_once(&self) {
        // Read what we need, then let go of the lock before any Win32 call (see `applying`).
        let (desired, hidden) = {
            let s = self.lock();
            (
                bar_rect(&s.current.rect, s.current.scale, s.settings.bar_height),
                s.hidden_for_fullscreen,
            )
        };

        // A failed ABM_NEW at startup (or a lost registration) is retried on the next apply.
        if !appbar::is_registered() && !appbar::register(self.hwnd()) {
            eprintln!("barista: appbar registration failed, will retry");
        }
        let granted = appbar::set_pos(self.hwnd(), desired);

        let changed = {
            let mut s = self.lock();
            let changed = s.last_rect != Some(granted) || s.last_visible == hidden;
            s.last_rect = Some(granted);
            s.last_visible = !hidden;
            changed
        };
        if changed {
            window::place(self.hwnd(), granted);
            window::set_visible(self.hwnd(), !hidden);
        }

        let info = {
            let mut s = self.lock();
            let info = Self::info_of(&s);
            if s.last_info.as_ref() == Some(&info) {
                return;
            }
            s.last_info = Some(info.clone());
            info
        };
        let _ = BarChanged(info).emit(&self.app);
    }

    /// Gives the screen space back. Called when the app exits.
    pub fn shutdown(&self) {
        appbar::unregister();
    }
}
