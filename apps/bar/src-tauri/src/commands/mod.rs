//! Tauri commands the frontend can call. Each one is listed in `specta_builder()` (lib.rs)
//! and appears in the generated `commands` object in bindings.ts.

use crate::appearance::BarAppearance;
use crate::bar::BarInfo;
use crate::controller;
use crate::radius::WindowRadius;
use crate::window_radius;
use crate::registry::{ClickError, Registry, StatusItemView};
use barista_protocol::MouseButton;
use std::sync::Arc;
use tauri::State;

/// The bar's version, from Cargo.toml. A sample command that proves the bridge works.
#[tauri::command]
#[specta::specta]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Where the bar is: height, monitor, DPI scale, all monitors, and whether it is hidden
/// because a full-screen app is in front.
#[tauri::command]
#[specta::specta]
pub fn get_bar_info() -> Result<BarInfo, String> {
    controller::get()
        .map(|c| c.info())
        .ok_or_else(|| "the bar is not running yet".into())
}

/// Moves the bar to another monitor (an id from `getBarInfo().monitors`) and remembers it.
#[tauri::command]
#[specta::specta]
pub fn set_monitor(id: String) -> Result<BarInfo, String> {
    controller::get()
        .ok_or("the bar is not running yet")?
        .set_monitor(&id)
}

/// The items in the bar right now. Call once when the UI starts; `itemsChanged` keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_items(registry: State<'_, Arc<Registry>>) -> Vec<StatusItemView> {
    registry.snapshot()
}

/// The user clicked an item. `id` is the item's `id` from `getItems()` / `itemsChanged`.
/// The click is sent to the app that owns the item (it gets `item.click`).
#[tauri::command]
#[specta::specta]
pub fn item_clicked(
    registry: State<'_, Arc<Registry>>,
    id: String,
    button: MouseButton,
) -> Result<(), String> {
    registry.click(&id, button).map_err(|e| match e {
        ClickError::NotFound => "that item is gone".to_string(),
        ClickError::Unreachable => "the app is not responding".to_string(),
    })
}

/// Is the wallpaper behind the bar light or dark? `light` means the bar draws dark content.
/// Call once when the UI starts; `barAppearanceChanged` keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_bar_appearance() -> BarAppearance {
    crate::appearance::current()
}

/// The corner radius of the window that calls this, in logical pixels (the same value is set
/// as the CSS variable `--window-radius` on that window). `radius` is `None` until the window
/// is known to the radius tracker.
#[tauri::command]
#[specta::specta]
pub fn get_window_radius(window: tauri::WebviewWindow) -> WindowRadius {
    window_radius::current(window.label())
}
