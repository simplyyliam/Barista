//! Tauri commands the frontend can call. Each one is listed in `specta_builder()` (lib.rs)
//! and appears in the generated `commands` object in bindings.ts.

/// The bar's version, from Cargo.toml. A sample command that proves the bridge works.
#[tauri::command]
#[specta::specta]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
