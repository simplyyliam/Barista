//! Commands of the settings window (ADR 0029). Each one validates through `Settings::clamped`,
//! saves, and answers with the `settingsChanged` event carrying the whole tree.

use crate::controller;
use crate::events::SettingsChanged;
use crate::settings::{
    Advanced, Appearance, BarItemPref, Behavior, ClockPrefs, ControlCenterPrefs, Section, Settings,
    TrayPrefs,
};
use tauri_specta::Event;

fn current() -> Settings {
    let mut s = controller::get().map(|c| c.settings()).unwrap_or_default();
    // The registry is the truth for autostart (Task Manager can switch it off).
    s.general.start_with_windows = crate::platform::windows::autostart::is_enabled();
    s
}

/// Changes and saves a copy of the settings, then tells every window.
pub fn change(app: &tauri::AppHandle, f: impl FnOnce(&mut Settings)) -> Settings {
    let mut s = match controller::get() {
        Some(c) => c.change_settings(f),
        None => Settings::default(),
    };
    s.general.start_with_windows = crate::platform::windows::autostart::is_enabled();
    let _ = SettingsChanged(s.clone()).emit(app);
    s
}

/// Tells every window the saved settings changed (for commands that save through other paths).
pub fn notify(app: &tauri::AppHandle) {
    let _ = SettingsChanged(current()).emit(app);
}

/// All saved settings, grouped like the settings window's sections.
#[tauri::command]
#[specta::specta]
pub fn get_settings() -> Settings {
    current()
}

/// Opens the settings window (or focuses it).
#[tauri::command]
#[specta::specta]
pub fn open_settings_window(app: tauri::AppHandle) {
    crate::settings_window::open(&app);
}

/// Start Barista when Windows starts (a value in the user's Run key). `Err` if the registry refused.
#[tauri::command]
#[specta::specta]
pub fn set_start_with_windows(app: tauri::AppHandle, on: bool) -> Result<(), String> {
    if !crate::platform::windows::autostart::set_enabled(on) {
        return Err("Windows would not change the startup entry".into());
    }
    change(&app, |s| s.general.start_with_windows = on);
    Ok(())
}

/// Tone, blur, font, text size, icon style and background of the bar and its menus.
#[tauri::command]
#[specta::specta]
pub fn set_appearance(app: tauri::AppHandle, appearance: Appearance) {
    change(&app, |s| s.menu_bar.appearance = appearance);
    crate::appearance::settings_changed(&app);
}

/// The bar's own items: order (the list's order), visibility and what each shows.
#[tauri::command]
#[specta::specta]
pub fn set_menu_bar_items(app: tauri::AppHandle, items: Vec<BarItemPref>) {
    change(&app, |s| s.menu_bar.items = items);
}

/// How the clock and date read.
#[tauri::command]
#[specta::specta]
pub fn set_clock_prefs(app: tauri::AppHandle, clock: ClockPrefs) {
    change(&app, |s| s.menu_bar.clock = clock);
}

/// The control center's tiles, sliders, cards and quick actions.
#[tauri::command]
#[specta::specta]
pub fn set_control_center_prefs(app: tauri::AppHandle, prefs: ControlCenterPrefs) {
    change(&app, |s| s.control_center = prefs);
}

/// Behaviour switches (hide on full screen).
#[tauri::command]
#[specta::specta]
pub fn set_behavior(app: tauri::AppHandle, behavior: Behavior) {
    change(&app, |s| s.behavior = behavior);
    if let Some(c) = controller::get() {
        c.behavior_changed();
    }
}

/// Tray options (style, hide the icons Explorer itself owns).
#[tauri::command]
#[specta::specta]
pub fn set_tray_prefs(
    app: tauri::AppHandle,
    registry: tauri::State<'_, std::sync::Arc<crate::registry::Registry>>,
    prefs: TrayPrefs,
) {
    registry.set_hide_windows_icons(prefs.hide_windows_icons);
    let style_changed =
        controller::get().is_some_and(|c| c.settings().menu_bar.tray.style != prefs.style);
    if style_changed {
        crate::panel::close();
    }
    let style = prefs.style;
    change(&app, |s| s.menu_bar.tray = prefs);
    if style_changed {
        let _ = crate::events::TrayStyleChanged(style).emit(&app);
    }
}

/// Windows' own taskbar auto-hides while Barista runs. Put back when switched off or on quit.
#[tauri::command]
#[specta::specta]
pub fn set_taskbar_auto_hide(app: tauri::AppHandle, on: bool) {
    change(&app, |s| s.menu_bar.taskbar.auto_hide = on);
    if let Some(c) = controller::get() {
        c.apply_taskbar();
        // The work area changed: reserve the bar's strip again.
        c.apply();
    }
}

/// Switches `barista | windows` for notifications, control center and the tray together
/// (the Advanced page). Each one also keeps its own command (rule 11).
#[tauri::command]
#[specta::specta]
pub fn set_advanced(app: tauri::AppHandle, advanced: Advanced) {
    let before = current().advanced;
    apply_advanced(&app, &before, &advanced);
}

/// Does what switching each `barista | windows` setting means, for the ones that differ.
fn apply_advanced(app: &tauri::AppHandle, before: &Advanced, now: &Advanced) {
    if now.notifications != before.notifications {
        crate::panel::close();
        crate::system::notifications::set_mode(now.notifications);
        let _ = crate::events::NotificationModeChanged(now.notifications).emit(app);
    }
    if now.control_center != before.control_center {
        crate::panel::close();
        crate::system::set_control_center_mode(now.control_center);
        let _ = crate::events::ControlCenterModeChanged(now.control_center).emit(app);
    }
    if now.tray != before.tray {
        crate::panel::close();
        change(app, |s| s.advanced.tray = now.tray);
        crate::tray::set_mode(now.tray);
    }
    notify(app);
}

/// Puts one section (or everything) back to its defaults.
#[tauri::command]
#[specta::specta]
pub fn reset_settings(app: tauri::AppHandle, section: Section) {
    let before = current();
    change(&app, |s| s.reset(section));
    apply_differences(&app, &before);
}

/// The settings as JSON text for a backup file. Machine-specific bits (monitor id, the saved
/// taskbar state) are left out.
#[tauri::command]
#[specta::specta]
pub fn export_settings() -> Result<String, String> {
    let mut s = current();
    s.general.monitor = None;
    s.menu_bar.taskbar.saved_state = None;
    s.notifications_seen_at = 0;
    serde_json::to_string_pretty(&s).map_err(|e| e.to_string())
}

/// Replaces the settings with a backup. A file that is not settings JSON is refused as a whole;
/// values that parse are cleaned and clamped like a hand-edited file. The monitor and the
/// notification stamp stay as they are.
#[tauri::command]
#[specta::specta]
pub fn import_settings(app: tauri::AppHandle, text: String) -> Result<(), String> {
    // A settings file is a few KB; refuse anything clearly not one (rule 3).
    if text.len() > 256 * 1024 {
        return Err("That file is too big to be a settings backup".into());
    }
    let imported = Settings::from_json(&text).ok_or("That file is not a Barista settings backup")?;
    let before = current();
    change(&app, |s| {
        let monitor = s.general.monitor.take();
        let seen = s.notifications_seen_at;
        let saved = s.menu_bar.taskbar.saved_state;
        *s = imported;
        s.general.monitor = monitor;
        s.notifications_seen_at = seen;
        s.menu_bar.taskbar.saved_state = saved;
    });
    apply_differences(&app, &before);
    Ok(())
}

/// After a bulk change (reset, import): do what the individual setters would have done.
fn apply_differences(app: &tauri::AppHandle, before: &Settings) {
    let after = current();
    if before.general.start_with_windows != after.general.start_with_windows {
        let _ = crate::platform::windows::autostart::set_enabled(after.general.start_with_windows);
        notify(app);
    }
    if before.menu_bar.taskbar.auto_hide != after.menu_bar.taskbar.auto_hide {
        if let Some(c) = controller::get() {
            c.apply_taskbar();
            c.apply();
        }
    }
    if before.menu_bar.height != after.menu_bar.height {
        if let Some(c) = controller::get() {
            c.set_bar_height(after.menu_bar.height);
        }
    }
    apply_advanced(app, &before.advanced, &after.advanced);
    crate::appearance::settings_changed(app);
}
