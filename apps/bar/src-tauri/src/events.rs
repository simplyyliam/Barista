//! Typed events the UI listens to (`bar-changed`, `items-changed`, `window-radius-changed`,
//! `menu-open`, `menu-changed`, `menu-close`).
//! Each event is a struct deriving `tauri_specta::Event`, so it lands in bindings.ts too.

use crate::bar::{BarInfo, Rect};
use crate::radius::WindowRadius;
use crate::registry::anchor::SubmenuSide;
use crate::registry::menu::MenuEntry;
use crate::registry::{Registry, StatusItemView};
use serde::{Deserialize, Serialize};
use specta::Type;

/// The bar moved, resized, changed monitor or hid/showed itself. Carries the new state.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct BarChanged(pub BarInfo);

/// The set of items in the bar changed. Carries the full list (visible items, in order), so
/// the UI just replaces what it has. Sent at most about once per frame (see `forward_changes`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct ItemsChanged(pub Vec<StatusItemView>);

/// Turns registry changes into `items-changed` events.
pub fn forward_changes(app: tauri::AppHandle, registry: std::sync::Arc<Registry>) {
    use tauri_specta::Event;
    tauri::async_runtime::spawn(async move {
        loop {
            registry.wait_changed().await;
            // Wait one frame, then send one snapshot: a client that updates 1000 times a
            // second makes the UI redraw at most ~60 times a second.
            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
            let _ = ItemsChanged(registry.snapshot()).emit(&app);
            if let Some(menu) = crate::menu_window::get() {
                menu.sync();
            }
        }
    });
}

/// A window's corner radius changed (it changed state, or Windhawk's settings changed). Sent
/// only to that window, only when its value really differs from the last one. `label` is the
/// window it is for, in case a listener receives events for several windows.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct WindowRadiusChanged {
    pub label: String,
    pub radius: WindowRadius,
}

/// Where a menu hangs from.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MenuAnchor {
    /// The item's rect on screen, in physical pixels.
    pub item: Rect,
    pub monitor_id: String,
    pub scale: f64,
    /// Draw an open submenu on this side (it would leave the screen on the other one).
    pub submenu_side: SubmenuSide,
}

/// A menu opened (or replaced another one): draw `menu` in the menu window. Sent to every
/// window; the menu window draws it, the bar can highlight `itemId`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct MenuOpen {
    pub item_id: String,
    pub menu: Vec<MenuEntry>,
    pub anchor: MenuAnchor,
}

/// The open menu's entries changed (the app sent a new menu, or a checkbox flipped). Redraw
/// without moving or closing.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct MenuChanged {
    pub item_id: String,
    pub menu: Vec<MenuEntry>,
}

/// The menu closed (blur, Esc, a pick, the same icon again, or its app went away).
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct MenuClose {
    pub item_id: String,
}
