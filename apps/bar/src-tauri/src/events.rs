//! Typed events the UI listens to (`bar-changed`, `items-changed`; `menu-open`, ... later).
//! Each event is a struct deriving `tauri_specta::Event`, so it lands in bindings.ts too.

use crate::bar::BarInfo;
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
        }
    });
}
