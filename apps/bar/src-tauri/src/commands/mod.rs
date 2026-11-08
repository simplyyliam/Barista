//! Tauri commands the frontend can call. Each one is listed in `specta_builder()` (lib.rs)
//! and appears in the generated `commands` object in bindings.ts.

use crate::bar::BarInfo;
use crate::controller;

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
    controller::get().map(|c| c.info()).ok_or_else(|| "the bar is not running yet".into())
}

/// Moves the bar to another monitor (an id from `getBarInfo().monitors`) and remembers it.
#[tauri::command]
#[specta::specta]
pub fn set_monitor(id: String) -> Result<BarInfo, String> {
    controller::get().ok_or("the bar is not running yet")?.set_monitor(&id)
}
