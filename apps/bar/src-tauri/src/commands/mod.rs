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
    // Item ids are "<session>/<id>", short; refuse anything absurd before looking it up.
    if item_id.len() > 256 {
        return;
    }
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

/// The item whose popover is open right now (the `id` from `getItems()`), if any. Call once when
/// the UI starts; `popoverChanged` keeps it fresh. Show that item as "open".
#[tauri::command]
#[specta::specta]
pub fn get_open_popover() -> Option<String> {
    crate::popover::get().and_then(|p| p.current())
}

/// Where each item is drawn in the bar (logical px, relative to the bar window), so menus open
/// right under their icon. Send it whenever the items move (a ResizeObserver is enough).
/// Without it, menus open under the mouse.
#[tauri::command]
#[specta::specta]
pub fn set_item_bounds(bounds: Vec<ItemBounds>) {
    crate::placement::set_item_bounds(bounds);
}

/// How the user wants the clock shown: 12 or 24 hour, first day of the week, time zone. The time
/// itself comes from JS. Call once when the UI starts; `clockSettingsChanged` keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_clock_settings() -> crate::system::clock::ClockSettings {
    crate::system::clock::service().get()
}

/// The battery right now. `present` is false on a desktop PC. Call once when the UI starts;
/// `batteryChanged` keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_battery() -> crate::system::battery::Battery {
    crate::system::battery::service().get()
}

/// The output volume right now. `available` is false when there is no output device. Call once
/// when the UI starts; `volumeChanged` keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_volume() -> crate::system::volume::Volume {
    crate::system::volume::service().get()
}

/// Sets the output volume (0 to 1; values outside are clamped, NaN is ignored). `volumeChanged`
/// follows with the real value.
#[tauri::command]
#[specta::specta]
pub fn set_volume(level: f64) {
    crate::system::volume::set_volume(level);
}

/// Mutes or unmutes the output. `volumeChanged` follows.
#[tauri::command]
#[specta::specta]
pub fn set_muted(muted: bool) {
    crate::system::volume::set_muted(muted);
}

/// Opens the panel `name` (a route like `control-center`) under bar item `anchor` (an id sent
/// with `setItemBounds`). Clicking the same anchor again closes it, another anchor moves it. The
/// panel window then follows `panelChanged`. `width` and `height` (logical px) are a first guess;
/// call `panelResized` once the content is measured. Opening closes any open menu or popover.
#[tauri::command]
#[specta::specta]
pub fn open_panel(
    name: String,
    anchor: String,
    width: f64,
    height: f64,
    chrome: crate::panel::PanelChrome,
) {
    if let Some(panel) = crate::panel::get() {
        panel.open(&name, &anchor, (clean_size(width), clean_size(height)), chrome);
    }
}

/// Closes the panel (Esc, or the UI decided). Does nothing if none is open.
#[tauri::command]
#[specta::specta]
pub fn close_panel() {
    crate::panel::close();
}

/// The panel window measured its content (logical px). The bar sizes and moves the window to fit
/// under its anchor and, the first time after an open, shows it.
#[tauri::command]
#[specta::specta]
pub fn panel_resized(width: f64, height: f64) {
    if let Some(panel) = crate::panel::get() {
        panel.resized(clean_size(width), clean_size(height));
    }
}

/// The panel open right now, if any. The panel window calls this once when it loads, then
/// follows `panelChanged`.
#[tauri::command]
#[specta::specta]
pub fn get_open_panel() -> Option<crate::panel::PanelOpen> {
    crate::panel::get().and_then(|p| p.current())
}

/// The notifications right now, grouped by app, newest first. Call once when the UI starts;
/// `notificationsChanged` keeps it fresh. Empty in `windows` mode.
#[tauri::command]
#[specta::specta]
pub fn get_notifications() -> crate::system::notifications::Notifications {
    crate::system::notifications::service().get()
}

/// Removes one notification, here and in Windows' own list.
#[tauri::command]
#[specta::specta]
pub fn dismiss_notification(id: u32) {
    crate::system::notifications::dismiss(id);
}

/// Removes every notification, here and in Windows' own list.
#[tauri::command]
#[specta::specta]
pub fn clear_notifications() {
    crate::system::notifications::clear();
}

/// A click on a notification: starts (or focuses) the app that sent it and removes the notification.
#[tauri::command]
#[specta::specta]
pub async fn open_notification(id: u32) {
    crate::system::notifications::open(id);
}

/// The notification's picture as a data URL. `None` when it has none or it can't be read; only
/// ask when `hasImage` is true.
#[tauri::command]
#[specta::specta]
pub async fn get_notification_image(id: u32) -> Option<String> {
    crate::system::notifications::image(id)
}

/// An app's icon as a PNG data URL (`appId` from a notification group). `None`: draw a letter.
#[tauri::command]
#[specta::specta]
pub async fn get_notification_app_icon(app_id: String) -> Option<String> {
    crate::system::notifications::app_icon(&app_id)
}

/// The notification panel was opened: clears the unread dot.
#[tauri::command]
#[specta::specta]
pub fn mark_notifications_seen() {
    crate::system::notifications::mark_seen();
}

/// Opens Windows' own notification access settings (for the "turn on access" state).
#[tauri::command]
#[specta::specta]
pub fn open_notification_settings() {
    crate::platform::windows::shell::open_uri("ms-settings:privacy-notifications");
}

/// Opens Windows' Focus settings (the Focus button; Windows has no way to start Focus for us).
#[tauri::command]
#[specta::specta]
pub fn open_focus_settings() {
    crate::platform::windows::shell::open_uri("ms-settings:quiethours");
}

/// Opens Windows' own notification center (what the date does in `windows` mode).
#[tauri::command]
#[specta::specta]
pub fn open_windows_notifications() {
    crate::platform::windows::shell::open_uri("ms-actioncenter:");
}

/// Who draws the notification panel: our own or Windows' (the v0.8 settings window flips this).
#[tauri::command]
#[specta::specta]
pub fn get_notification_mode() -> crate::settings::Mode {
    crate::system::notifications::mode()
}

/// Switches the notification panel between ours and Windows'. Saved; `notificationModeChanged` follows.
#[tauri::command]
#[specta::specta]
pub fn set_notification_mode(app: tauri::AppHandle, mode: crate::settings::Mode) {
    use tauri_specta::Event;
    if mode == crate::system::notifications::mode() {
        return;
    }
    crate::panel::close();
    crate::system::notifications::set_mode(mode);
    let _ = crate::events::NotificationModeChanged(mode).emit(&app);
}

/// Runs a control center action. Restart, shut down and sign out only run with `confirm: true`
/// (the UI asks the user first); without it they return `needsConfirm` and do nothing.
#[tauri::command]
#[specta::specta]
pub fn run_action(
    action: crate::system::actions::Action,
    confirm: bool,
) -> Result<(), crate::system::actions::ActionError> {
    crate::system::actions::run(action, confirm)
}

/// Opens one of Windows' own settings pages (the fallback for everything the bar can't toggle itself).
#[tauri::command]
#[specta::specta]
pub fn open_settings(page: crate::system::actions::SettingsPage) {
    crate::system::actions::open_settings(page);
}

/// Bluetooth right now: radio state and paired devices. `available` is false without a radio. Call
/// once when the UI starts; `bluetoothChanged` keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_bluetooth() -> crate::system::bluetooth::Bluetooth {
    crate::system::bluetooth::service().get()
}

/// Turns the Bluetooth radio on or off. `bluetoothChanged` follows. Connecting a single device has
/// no public API: open the Bluetooth settings page for that.
#[tauri::command]
#[specta::specta]
pub async fn set_bluetooth_enabled(enabled: bool) {
    crate::system::bluetooth::set_enabled(enabled);
}

/// Wi-Fi right now. `available` is false without an adapter. Call once when the UI starts;
/// `wifiChanged` keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_wifi() -> crate::system::wifi::Wifi {
    crate::system::wifi::service().get()
}

/// Turns the Wi-Fi radio on or off. `wifiChanged` follows.
#[tauri::command]
#[specta::specta]
pub async fn set_wifi_enabled(enabled: bool) {
    crate::system::wifi::set_enabled(enabled);
}

/// Asks Windows to look for networks again. `wifiChanged` follows when the scan is done.
#[tauri::command]
#[specta::specta]
pub fn scan_wifi() {
    crate::system::wifi::scan();
}

/// Joins a network Windows already knows. For any other name this opens Windows' own Wi-Fi
/// flyout (joining a new secured network is not done by the bar).
#[tauri::command]
#[specta::specta]
pub fn connect_wifi(ssid: String) {
    crate::system::wifi::connect(&ssid);
}

/// Disconnects from the current network.
#[tauri::command]
#[specta::specta]
pub fn disconnect_wifi() {
    crate::system::wifi::disconnect();
}

/// Airplane mode right now. The bar can read it but not switch it: open the airplane mode
/// settings page (`openSettings`) for that. Call once when the UI starts; `airplaneModeChanged`
/// keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_airplane_mode() -> crate::system::airplane::AirplaneMode {
    crate::system::airplane::service().get()
}

/// The active output devices and which one is the default. Call once when the UI starts;
/// `outputDevicesChanged` keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_output_devices() -> crate::system::output_devices::OutputDevices {
    crate::system::output_devices::service().get()
}

/// Makes an output device the default (`id` from `getOutputDevices`). When `canSwitch` is false,
/// or the id is stale, this opens Windows' sound settings instead.
#[tauri::command]
#[specta::specta]
pub fn set_output_device(id: String) {
    crate::system::output_devices::set_default(id);
}

/// The signed-in user's display name and picture (data URL). Call once when the UI starts;
/// `userChanged` follows when the first read finishes.
#[tauri::command]
#[specta::specta]
pub fn get_user() -> crate::system::user::User {
    crate::system::user::service().get()
}

/// The mobile hotspot right now. `supported` is false when this PC can't share its connection.
/// Call once when the UI starts; `hotspotChanged` keeps it fresh (also read again when the
/// control center opens).
#[tauri::command]
#[specta::specta]
pub fn get_hotspot() -> crate::system::hotspot::Hotspot {
    crate::system::hotspot::service().get()
}

/// Starts or stops the hotspot. The state is `changing` for a few seconds, then `hotspotChanged`
/// shows the result.
#[tauri::command]
#[specta::specta]
pub async fn set_hotspot_enabled(enabled: bool) {
    crate::system::hotspot::set_enabled(enabled);
}

/// The brightness of every display: the laptop panel first, then external monitors. A display
/// with `supported: false` can't be controlled. Call once when the UI starts; `brightnessChanged`
/// keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_brightness() -> crate::system::brightness::Brightness {
    crate::system::brightness::service().get()
}

/// Sets one display's brightness (`id` from `getBrightness`, `level` 0 to 1, never below 1%).
/// Unknown ids and unsupported displays are ignored. `brightnessChanged` follows.
#[tauri::command]
#[specta::specta]
pub fn set_brightness(id: String, level: f64) {
    crate::system::brightness::set(&id, level);
}

/// Who draws the control center (Wi-Fi, Bluetooth, brightness, ...): ours or Windows' Quick
/// Settings (the v0.8 settings window flips this).
#[tauri::command]
#[specta::specta]
pub fn get_control_center_mode() -> crate::settings::Mode {
    crate::system::control_center_mode()
}

/// Switches the control center between ours and Windows'. Saved; `controlCenterModeChanged` follows.
/// In `windows` mode the bar's Wi-Fi, Bluetooth and brightness state go empty and the UI should call
/// `openWindowsControlCenter` instead of opening its own panel.
#[tauri::command]
#[specta::specta]
pub fn set_control_center_mode(app: tauri::AppHandle, mode: crate::settings::Mode) {
    use tauri_specta::Event;
    if mode == crate::system::control_center_mode() {
        return;
    }
    crate::panel::close();
    crate::system::set_control_center_mode(mode);
    let _ = crate::events::ControlCenterModeChanged(mode).emit(&app);
}

/// Opens Windows' own Quick Settings (what the control center item does in `windows` mode).
#[tauri::command]
#[specta::specta]
pub fn open_windows_control_center() {
    crate::platform::windows::shell::open_uri("ms-actioncenter:controlcenter/&showFooter=true");
}
