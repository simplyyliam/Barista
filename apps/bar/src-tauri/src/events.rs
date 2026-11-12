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
    // Menu-only changes (new menu, checkbox flip) don't change the bar, so they only refresh an
    // open menu and skip `items-changed`.
    let menus = registry.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            menus.wait_menu_changed().await;
            if let Some(menu) = crate::menu_window::get() {
                menu.sync();
            }
        }
    });
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
            if let Some(popovers) = crate::popover::get() {
                popovers.sync();
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
    /// True when the window has the system acrylic backdrop. False (Windows 10): draw an opaque panel.
    pub acrylic: bool,
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

/// The popover that is open changed: `item_id` is the item it hangs from, `None` = none is open.
/// The UI shows the item as "open" while this is set.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct PopoverChanged {
    pub item_id: Option<String>,
}

/// Draw `entries` in the submenu window (the entry `index` of the open menu was opened). The
/// window shows once the UI reports its size (`submenuResized`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SubmenuShow {
    pub item_id: String,
    pub index: u32,
    pub entries: Vec<MenuEntry>,
}

/// The submenu closed (the main menu draws nothing for it any more).
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SubmenuHide {
    pub item_id: String,
}

/// Where the submenu window landed, in logical px relative to the menu window's top-left corner.
/// The main menu uses it for the "safe triangle" (diagonal mouse moves toward the submenu).
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SubmenuPlaced {
    pub side: SubmenuSide,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Highlight this row in the submenu (keyboard moved it), or none (`null`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SubmenuHighlight {
    pub index: Option<u32>,
}

/// The mouse is over this submenu row (or left the submenu: `null`). The main menu follows it,
/// so mouse and keyboard share one highlight.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct SubmenuHovered {
    pub index: Option<u32>,
}

/// The clock settings changed (12/24 h, first weekday, time zone). Carries the full snapshot, at
/// most once per frame.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct ClockSettingsChanged(pub crate::system::clock::ClockSettings);

/// The battery changed (level, charging, power source, battery saver). Full snapshot, at most
/// once per frame.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct BatteryChanged(pub crate::system::battery::Battery);

/// The volume or mute of the output device changed, or the default device did. Full snapshot, at
/// most once per frame.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct VolumeChanged(pub crate::system::volume::Volume);

/// The notifications changed (arrived, dismissed, unread, access). Full snapshot, at most once per
/// frame.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct NotificationsChanged(pub crate::system::notifications::Notifications);

/// The notification panel is now ours or Windows' (a setting, see `setNotificationMode`).
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct NotificationModeChanged(pub crate::settings::Mode);

/// Startup only: the panel window is being warmed up (shown invisibly) while this is `true`. The
/// panel page draws its content meanwhile, so the first real open is as fast as the next ones.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct PanelWarm(pub bool);

/// The panel opened, moved to another item or closed (`None`). Carries what the UI needs: which
/// route to show (`name`), which item is open (`anchor`) and where the window landed. Sent to
/// every window; the panel window draws the route, the bar can show `anchor` as open.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct PanelChanged {
    pub open: Option<crate::panel::PanelOpen>,
}

/// Bluetooth changed (radio on or off, a paired device connected or removed). Full snapshot, at
/// most once per frame.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct BluetoothChanged(pub crate::system::bluetooth::Bluetooth);

/// Wi-Fi changed (radio, current network, networks in range). Full snapshot, at most once per
/// frame.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct WifiChanged(pub crate::system::wifi::Wifi);
