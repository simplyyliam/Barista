//! Typed events the UI listens to (`bar-changed` now; `items-changed`, `menu-open`, ... later).
//! Each event is a struct deriving `tauri_specta::Event`, so it lands in bindings.ts too.

use crate::bar::BarInfo;
use serde::{Deserialize, Serialize};
use specta::Type;

/// The bar moved, resized, changed monitor or hid/showed itself. Carries the new state.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct BarChanged(pub BarInfo);
