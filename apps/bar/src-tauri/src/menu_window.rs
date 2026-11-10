//! The menu window: one hidden, transparent window made at startup and reused for every menu
//! (ADR 0010). Rust decides when it opens, where it goes and when it closes; the UI only draws
//! the entries it gets in `menu-open` and reports its size.
//!
//! ```text
//!  click icon ──► route_click ──► open: place under icon, show, focus, emit menu-open
//!  UI measures ─► menuResized ──► re-place (flip submenu, shift at edges)
//!  pick entry ──► menuItemClicked ─► registry flips checked, app gets menu.click ─► close
//!  blur / Esc / same icon again ─► close (emit menu-close)
//! ```

use crate::bar::Rect;
use crate::events::{MenuAnchor, MenuChanged, MenuClose, MenuOpen};
use crate::registry::anchor::{self, ItemBounds, SubmenuSide};
use crate::registry::{ClickError, MenuItemClickError, MenuSnapshot, Registry};
use barista_protocol::{MenuItemType, MouseButton};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use tauri_specta::Event;

pub const LABEL: &str = "menu";

/// After the menu loses focus, wait this long before hiding it. If a click on a bar icon
/// arrives in between, it decides instead: same icon = stay closed, another icon = switch
/// menus without hiding (no flicker).
const BLUR_GRACE: Duration = Duration::from_millis(150);

/// Size guess (logical px) until the UI reports the real one.
const GUESS_WIDTH: f64 = 220.0;
const GUESS_ROW: f64 = 24.0;
const GUESS_SEPARATOR: f64 = 9.0;
const GUESS_PADDING: f64 = 10.0;

/// Reply to `menuResized`: which side the UI should draw an open submenu on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MenuPlacement {
    pub submenu_side: SubmenuSide,
}

/// What the open/close logic decided. Kept apart from Tauri so it can be tested.
#[derive(Debug, PartialEq, Eq)]
enum Action {
    Nothing,
    Open(String),
    /// Another item's menu replaces the open one; the window stays up.
    Swap(String),
    Close,
}

#[derive(Default)]
struct Machine {
    /// The item whose menu is showing.
    open: Option<String>,
    /// Set when the window lost focus while open.
    blurred_at: Option<Instant>,
    /// The item whose menu the blur timer just closed, and when.
    closed_by_blur: Option<(String, Instant)>,
}

impl Machine {
    fn click(&mut self, key: &str, now: Instant) -> Action {
        let recent = |t: Instant| now.duration_since(t) < BLUR_GRACE;
        match (&self.open, self.blurred_at) {
            (Some(open), _) if open == key => {
                self.reset();
                Action::Close
            }
            (Some(_), _) => {
                self.open = Some(key.to_string());
                self.blurred_at = None;
                Action::Swap(key.to_string())
            }
            (None, _) => {
                // The blur closed this very menu a moment ago: the click was meant to close it.
                if let Some((closed, at)) = &self.closed_by_blur {
                    if closed == key && recent(*at) {
                        self.closed_by_blur = None;
                        return Action::Nothing;
                    }
                }
                self.open = Some(key.to_string());
                self.blurred_at = None;
                Action::Open(key.to_string())
            }
        }
    }

    fn blur(&mut self, now: Instant) {
        if self.open.is_some() {
            self.blurred_at = Some(now);
        }
    }

    fn focus(&mut self) {
        self.blurred_at = None;
    }

    /// The grace timer fired.
    fn blur_timeout(&mut self, now: Instant) -> Action {
        match (&self.open, self.blurred_at) {
            (Some(open), Some(at)) if now.duration_since(at) >= BLUR_GRACE => {
                self.closed_by_blur = Some((open.clone(), now));
                self.open = None;
                self.blurred_at = None;
                Action::Close
            }
            _ => Action::Nothing,
        }
    }

    fn reset(&mut self) {
        self.open = None;
        self.blurred_at = None;
    }
}

struct Inner {
    machine: Machine,
    /// The item whose menu the window shows right now (the machine may already have moved on,
    /// e.g. to closed, before the window is hidden).
    shown: Option<String>,
    /// Item rects from the UI (`setItemBounds`), logical and bar-relative, keyed like
    /// `s3/counter`. Converted to screen pixels only when a menu opens, with the bar's
    /// position and scale at that moment (it may have moved monitor since).
    bounds: HashMap<String, ItemBounds>,
    /// Rect of the item the open menu hangs from.
    item_rect: Rect,
    /// Last size the UI reported (logical px): width, height, root width.
    size: (f64, f64, f64),
    rev: u64,
    side: SubmenuSide,
}

pub struct MenuWindow {
    app: AppHandle,
    registry: Arc<Registry>,
    inner: Mutex<Inner>,
}

static MENU: OnceLock<MenuWindow> = OnceLock::new();

pub fn get() -> Option<&'static MenuWindow> {
    MENU.get()
}

/// Makes the hidden menu window. Call once from `setup`.
pub fn init(app: &AppHandle, registry: Arc<Registry>) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Barista menu")
        .visible(false)
        .focused(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .inner_size(GUESS_WIDTH, 100.0);
    // Same browser arguments as the main window, or WebView2 refuses to start (see CHECKPOINT).
    if let Some(args) = app
        .config()
        .app
        .windows
        .first()
        .and_then(|w| w.additional_browser_args.clone())
    {
        builder = builder.additional_browser_args(&args);
    }
    let window = builder.build()?;
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        crate::platform::windows::window::set_menu_styles(hwnd);
    }
    crate::window_radius::register(app, LABEL, crate::window_radius::Mode::Normal)?;
    window.on_window_event(|event| {
        if let tauri::WindowEvent::Focused(focused) = event {
            if let Some(menu) = get() {
                menu.focus_changed(*focused);
            }
        }
    });
    let _ = MENU.set(MenuWindow {
        app: app.clone(),
        registry,
        inner: Mutex::new(Inner {
            machine: Machine::default(),
            shown: None,
            bounds: HashMap::new(),
            item_rect: Rect::default(),
            size: (0.0, 0.0, 0.0),
            rev: 0,
            side: SubmenuSide::Right,
        }),
    });
    Ok(())
}

/// A click on a bar item: opens/closes its menu if it has one, else tells the app
/// (`item.click`). Used by the `itemClicked` command and by `debug.click`.
pub fn route_click(registry: &Registry, key: &str, button: MouseButton) -> Result<(), ClickError> {
    if !registry.opens_menu(key, button) {
        return registry.click(key, button);
    }
    // No window in unit tests: the click is accepted and nothing opens.
    if let Some(menu) = get() {
        menu.item_click(key);
    }
    Ok(())
}

/// The user picked an entry. Closes the menu if it is the open one.
pub fn menu_item_clicked(
    registry: &Registry,
    key: &str,
    menu_item_id: &str,
) -> Result<(), MenuItemClickError> {
    let result = registry.menu_click(key, menu_item_id);
    if matches!(result, Ok(()) | Err(MenuItemClickError::Unreachable)) {
        if let Some(menu) = get() {
            menu.close_if(key);
        }
    }
    result
}

/// Rough size of a menu (logical px), so the window can show before the UI measures it.
fn guess_size(snapshot: &MenuSnapshot) -> (f64, f64) {
    let height = snapshot.entries.iter().fold(GUESS_PADDING, |h, e| {
        h + if e.kind == MenuItemType::Separator {
            GUESS_SEPARATOR
        } else {
            GUESS_ROW
        }
    });
    (GUESS_WIDTH, height)
}

/// The bar's strip, the area menus may use, and the scale, from the controller.
fn geometry() -> Option<(Rect, Rect, f64)> {
    let info = crate::controller::get()?.info();
    let m = &info.monitor;
    let bar = Rect {
        left: m.rect.left,
        top: m.rect.top,
        right: m.rect.right,
        bottom: m.rect.top + info.height_px,
    };
    // The work area already starts under the bar once the AppBar is registered; make sure.
    let mut work = m.work_area;
    work.top = work.top.max(bar.bottom);
    Some((bar, work, info.scale))
}

impl MenuWindow {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn window(&self) -> Option<tauri::WebviewWindow> {
        self.app.get_webview_window(LABEL)
    }

    fn item_click(&self, key: &str) {
        let action = self.lock().machine.click(key, Instant::now());
        self.apply(action);
    }

    fn focus_changed(&self, focused: bool) {
        if focused {
            self.lock().machine.focus();
            return;
        }
        self.lock().machine.blur(Instant::now());
        tauri::async_runtime::spawn(async {
            tokio::time::sleep(BLUR_GRACE).await;
            if let Some(menu) = get() {
                let action = menu.lock().machine.blur_timeout(Instant::now());
                menu.apply(action);
            }
        });
    }

    /// `closeMenu` from the UI (Esc, a role it handled itself...).
    pub fn close(&self) {
        let shown = {
            let mut inner = self.lock();
            inner.machine.reset();
            inner.shown.clone()
        };
        if let Some(key) = shown {
            self.hide(&key);
        }
    }

    fn close_if(&self, key: &str) {
        if self.lock().shown.as_deref() == Some(key) {
            self.close();
        }
    }

    fn apply(&self, action: Action) {
        match action {
            Action::Nothing => {}
            Action::Open(key) | Action::Swap(key) => self.show(&key),
            // The machine already forgot which key; the event names the last one we showed.
            Action::Close => {
                let key = self.lock().last_key();
                self.hide(&key);
            }
        }
    }

    fn show(&self, key: &str) {
        // The item vanished between the click and now, or the bar has no monitor yet: close
        // whatever is showing, so the window and the state machine agree.
        let (Some(snapshot), Some((bar, work, scale))) = (self.registry.menu(key), geometry())
        else {
            self.close();
            return;
        };
        let (w, h) = guess_size(&snapshot);
        let (rect, side) = {
            let mut inner = self.lock();
            #[cfg(windows)]
            let cursor = crate::platform::windows::window::cursor_x();
            #[cfg(not(windows))]
            let cursor: Option<i32> = None;
            inner.item_rect = match inner.bounds.get(key) {
                Some(b) => anchor::to_screen(b, &bar, scale),
                None => anchor::at_cursor(cursor.unwrap_or(bar.left), &bar),
            };
            inner.size = (w, h, w);
            inner.rev = snapshot.rev;
            inner.shown = Some(key.to_string());
            let placed = place_logical(&inner.item_rect, inner.size, &work, scale);
            inner.side = placed.1;
            placed
        };
        let anchor = MenuAnchor {
            item: self.lock().item_rect,
            monitor_id: crate::controller::get()
                .map(|c| c.info().monitor.id)
                .unwrap_or_default(),
            scale,
            submenu_side: side,
        };
        if let Some(window) = self.window() {
            let _ = window.set_size(PhysicalSize::new(rect.width() as u32, rect.height() as u32));
            let _ = window.set_position(PhysicalPosition::new(rect.left, rect.top));
            let _ = window.show();
            // Focus so arrow keys, Enter and Esc reach the menu without touching the mouse.
            let _ = window.set_focus();
        }
        let _ = MenuOpen {
            item_id: key.to_string(),
            menu: snapshot.entries,
            anchor,
        }
        .emit(&self.app);
    }

    fn hide(&self, key: &str) {
        if key.is_empty() {
            return;
        }
        if let Some(window) = self.window() {
            let _ = window.hide();
        }
        self.lock().shown = None;
        let _ = MenuClose {
            item_id: key.to_string(),
        }
        .emit(&self.app);
    }

    /// `menuResized`: the UI measured itself (logical px). Re-places the window.
    pub fn resized(&self, width: f64, height: f64, root_width: f64) -> MenuPlacement {
        let Some((_, work, scale)) = geometry() else {
            return MenuPlacement {
                submenu_side: SubmenuSide::Right,
            };
        };
        let (rect, side) = {
            let mut inner = self.lock();
            if inner.shown.is_none() {
                return MenuPlacement {
                    submenu_side: inner.side,
                };
            }
            inner.size = (width, height, root_width);
            let placed = place_logical(&inner.item_rect, inner.size, &work, scale);
            inner.side = placed.1;
            placed
        };
        if let Some(window) = self.window() {
            let _ = window.set_size(PhysicalSize::new(rect.width() as u32, rect.height() as u32));
            let _ = window.set_position(PhysicalPosition::new(rect.left, rect.top));
        }
        MenuPlacement { submenu_side: side }
    }

    /// `setItemBounds`: where the UI drew each item (logical, relative to the bar window).
    pub fn set_item_bounds(&self, items: Vec<ItemBounds>) {
        self.lock().bounds = items
            .into_iter()
            .take(256) // the UI is trusted, but a runaway loop shouldn't grow this forever
            .map(|b| (b.id.clone(), b))
            .collect();
    }

    /// For `getOpenMenu`: what is showing right now.
    pub fn current(&self) -> Option<MenuOpen> {
        let (key, item, side) = {
            let inner = self.lock();
            (inner.shown.clone()?, inner.item_rect, inner.side)
        };
        let snapshot = self.registry.menu(&key)?;
        let info = crate::controller::get()?.info();
        Some(MenuOpen {
            item_id: key,
            menu: snapshot.entries,
            anchor: MenuAnchor {
                item,
                monitor_id: info.monitor.id,
                scale: info.scale,
                submenu_side: side,
            },
        })
    }

    /// After any registry change: redraw the open menu if it changed, close it if its item or
    /// menu is gone (client quit, `setMenu([])`, item hidden).
    pub fn sync(&self) {
        let Some((key, rev)) = self.lock().shown.clone().map(|k| {
            let rev = self.registry.menu_rev(&k);
            (k, rev)
        }) else {
            return;
        };
        // Only the cheap revision is read on every change; the menu is built when it differs.
        match rev {
            None => self.close(),
            Some(rev) if rev == self.lock().rev => {}
            Some(_) => {
                let Some(snapshot) = self.registry.menu(&key) else {
                    return self.close();
                };
                self.lock().rev = snapshot.rev;
                let _ = MenuChanged {
                    item_id: key,
                    menu: snapshot.entries,
                }
                .emit(&self.app);
            }
        }
    }
}

impl Inner {
    fn last_key(&self) -> String {
        self.shown.clone().unwrap_or_default()
    }
}

/// Logical size -> physical, then `anchor::place`.
fn place_logical(
    item: &Rect,
    size: (f64, f64, f64),
    work: &Rect,
    scale: f64,
) -> (Rect, SubmenuSide) {
    let px = |v: f64| (v * scale).round() as i32;
    anchor::place(item, (px(size.0), px(size.1)), px(size.2), work)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn later(t: Instant, ms: u64) -> Instant {
        t + Duration::from_millis(ms)
    }

    #[test]
    fn clicking_the_same_icon_toggles() {
        let mut m = Machine::default();
        let t = Instant::now();
        assert_eq!(m.click("s1/a", t), Action::Open("s1/a".into()));
        assert_eq!(m.click("s1/a", t), Action::Close);
        assert_eq!(m.click("s1/a", t), Action::Open("s1/a".into()));
    }

    #[test]
    fn clicking_another_icon_swaps_without_closing() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        assert_eq!(m.click("s2/b", t), Action::Swap("s2/b".into()));
        assert_eq!(m.open.as_deref(), Some("s2/b"));
    }

    #[test]
    fn blur_closes_after_the_grace_time() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        m.blur(t);
        assert_eq!(m.blur_timeout(later(t, 50)), Action::Nothing);
        assert_eq!(m.blur_timeout(later(t, 200)), Action::Close);
        assert!(m.open.is_none());
    }

    #[test]
    fn a_click_during_the_grace_time_wins_over_the_blur() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        m.blur(t);
        // Another icon: swap, and the pending blur is cancelled.
        assert_eq!(m.click("s2/b", later(t, 40)), Action::Swap("s2/b".into()));
        assert_eq!(m.blur_timeout(later(t, 200)), Action::Nothing);
        // Same icon during a blur: close.
        m.blur(later(t, 300));
        assert_eq!(m.click("s2/b", later(t, 320)), Action::Close);
    }

    #[test]
    fn same_icon_right_after_the_blur_closed_it_stays_closed() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        m.blur(t);
        assert_eq!(m.blur_timeout(later(t, 150)), Action::Close);
        assert_eq!(m.click("s1/a", later(t, 200)), Action::Nothing);
        // Later clicks open it again.
        assert_eq!(m.click("s1/a", later(t, 1000)), Action::Open("s1/a".into()));
    }

    #[test]
    fn focus_back_cancels_the_blur() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        m.blur(t);
        m.focus();
        assert_eq!(m.blur_timeout(later(t, 500)), Action::Nothing);
    }

    #[test]
    fn size_guess_counts_rows_and_separators() {
        use crate::registry::menu::MenuEntry;
        let row = |kind| MenuEntry {
            id: None,
            kind,
            label: String::new(),
            checked: false,
            enabled: true,
            accelerator: None,
            role: None,
            icon: None,
            submenu: vec![],
        };
        let snap = MenuSnapshot {
            entries: vec![
                row(MenuItemType::Normal),
                row(MenuItemType::Separator),
                row(MenuItemType::Normal),
            ],
            rev: 0,
        };
        assert_eq!(
            guess_size(&snap),
            (
                GUESS_WIDTH,
                GUESS_PADDING + 2.0 * GUESS_ROW + GUESS_SEPARATOR
            )
        );
    }
}
