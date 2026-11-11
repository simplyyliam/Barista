//! Tauri commands the frontend can call. Each one is listed in `specta_builder()` (lib.rs)
//! and appears in the generated `commands` object in bindings.ts.

use crate::appearance::BarAppearance;
use crate::bar::BarInfo;
use crate::controller;
use crate::events::MenuOpen;
use crate::menu_window;
use crate::radius::WindowRadius;
use crate::registry::anchor::ItemBounds;
use crate::registry::menu::MenuClickError;
use crate::registry::{ClickError, MenuItemClickError, Registry, StatusItemView};
use crate::window_radius;
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
/// If the item has a menu, a left or right click opens it (or closes it if it is open) and the
/// app is not told; otherwise the app gets `item.click`.
#[tauri::command]
#[specta::specta]
pub fn item_clicked(
    registry: State<'_, Arc<Registry>>,
    id: String,
    button: MouseButton,
) -> Result<(), String> {
    menu_window::route_click(&registry, &id, button).map_err(|e| match e {
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

/// The user picked entry `menu_item_id` in the menu of item `item_id`. Checkbox and radio
/// entries flip in the bar (a `menu-changed` follows), the app gets `menu.click`, and the menu
/// closes. Clicks on separators, submenus or disabled entries are refused.
#[tauri::command]
#[specta::specta]
pub fn menu_item_clicked(
    registry: State<'_, Arc<Registry>>,
    item_id: String,
    menu_item_id: String,
) -> Result<(), String> {
    menu_window::menu_item_clicked(&registry, &item_id, &menu_item_id).map_err(|e| match e {
        MenuItemClickError::NotFound | MenuItemClickError::Menu(MenuClickError::NotFound) => {
            "that menu entry is gone".to_string()
        }
        MenuItemClickError::Menu(MenuClickError::NotClickable) => {
            "that entry can't be clicked".to_string()
        }
        MenuItemClickError::Unreachable => "the app is not responding".to_string(),
    })
}

/// The menu window measured its panel (logical px). The bar sizes and moves the window to fit
/// on screen and, the first time after an open, shows it.
#[tauri::command]
#[specta::specta]
pub fn menu_resized(width: f64, height: f64) {
    if let Some(menu) = menu_window::get() {
        menu.resized(clean_size(width), clean_size(height));
    }
}

/// The submenu window measured its panel (logical px). The bar places it next to the menu
/// (flipping to the left near the screen edge) and shows it.
#[tauri::command]
#[specta::specta]
pub fn submenu_resized(width: f64, height: f64) {
    if let Some(menu) = menu_window::get() {
        menu.submenu_resized(clean_size(width), clean_size(height));
    }
}

/// Open the submenu of entry `index` of the open menu. `trigger_top` is that row's top in
/// logical px, relative to the menu window. The bar answers with `submenuShow` (draw this) and
/// `submenuPlaced` (it landed here). Does nothing if the entry has no (enabled) submenu.
#[tauri::command]
#[specta::specta]
pub fn open_submenu(index: u32, trigger_top: f64) {
    if let Some(menu) = menu_window::get() {
        let top = if trigger_top.is_finite() {
            trigger_top.clamp(0.0, 10_000.0)
        } else {
            0.0
        };
        menu.open_submenu(index as usize, top);
    }
}

/// Closes the submenu (instantly). Does nothing if none is open.
#[tauri::command]
#[specta::specta]
pub fn close_submenu() {
    if let Some(menu) = menu_window::get() {
        menu.close_submenu();
    }
}

/// The mouse is over row `index` of the submenu (or left it: `null`). The bar relays it to the
/// menu window as `submenuHovered`, so mouse and keyboard share one highlight.
#[tauri::command]
#[specta::specta]
pub fn submenu_hover(index: Option<u32>) {
    if let Some(menu) = menu_window::get() {
        menu.submenu_hover(index);
    }
}

/// Keyboard moved the highlight to row `index` of the submenu (or none). The bar relays it to
/// the submenu window as `submenuHighlight`.
#[tauri::command]
#[specta::specta]
pub fn set_submenu_highlight(index: Option<u32>) {
    if let Some(menu) = menu_window::get() {
        menu.set_submenu_highlight(index);
    }
}

/// The mouse moved onto bar item `item_id`. If another item's menu is open, it switches to this
/// one without a click. Does nothing when no menu is open (hover never opens a menu).
#[tauri::command]
#[specta::specta]
pub fn menu_hover(item_id: String) {
    if let Some(menu) = menu_window::get() {
        menu.hover_item(&item_id);
    }
}

fn clean_size(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(1.0, 10_000.0)
    } else {
        1.0
    }
}

/// Closes the open menu (Esc, or the UI decided). Does nothing if none is open.
#[tauri::command]
#[specta::specta]
pub fn close_menu() {
    if let Some(menu) = menu_window::get() {
        menu.close();
    }
}

/// The menu showing right now, if any. The menu window calls this once when it loads, then
/// follows `menuOpen` / `menuChanged` / `menuClose`.
#[tauri::command]
#[specta::specta]
pub fn get_open_menu() -> Option<MenuOpen> {
    menu_window::get().and_then(|m| m.current())
}

/// Where each item is drawn in the bar (logical px, relative to the bar window), so menus open
/// right under their icon. Send it whenever the items move (a ResizeObserver is enough).
/// Without it, menus open under the mouse.
#[tauri::command]
#[specta::specta]
pub fn set_item_bounds(bounds: Vec<ItemBounds>) {
    if let Some(menu) = menu_window::get() {
        menu.set_item_bounds(bounds);
    }
}
