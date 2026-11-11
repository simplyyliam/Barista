//! The menu windows: two hidden, transparent windows made at startup and reused for every menu
//! (ADR 0010, 0013). `menu` holds the main panel, `menu-sub` the open submenu. Each window is one
//! panel, because the system acrylic blur covers a whole window. Rust decides when they open,
//! where they go and when they close; the UI only draws the entries it gets and reports its size.
//!
//! ```text
//!  click/hover icon ─► route_click ─► open: place under icon, emit menu-open
//!  UI measures ───────► menuResized ─► re-place, then show + focus (first time only)
//!  hover a submenu row ► openSubmenu ─► emit submenu-show ─► submenuResized ─► place + show
//!  pick entry ────────► menuItemClicked ─► registry flips checked, app gets menu.click ─► close
//!  blur / Esc / same icon again ─► close: emit menu-close, hide both windows (Windows fades them)
//! ```

use crate::bar::Rect;
use crate::events::{
    MenuAnchor, MenuChanged, MenuClose, MenuOpen, SubmenuHide, SubmenuHighlight, SubmenuHovered,
    SubmenuPlaced, SubmenuShow,
};
use crate::registry::anchor::{self, ItemBounds};
use crate::registry::{ClickError, MenuItemClickError, MenuSnapshot, Registry};
use barista_protocol::{MenuItemType, MouseButton};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};
use tauri_specta::Event;

pub const LABEL: &str = "menu";
pub const SUB_LABEL: &str = "menu-sub";

/// After the menu loses focus, wait this long before hiding it. If a click on a bar icon
/// arrives in between, it decides instead: same icon = stay closed, another icon = switch
/// menus without hiding (no flicker).
const BLUR_GRACE: Duration = Duration::from_millis(150);

/// If the UI never reports its size (it should within a frame), show the window anyway.
const REVEAL_FALLBACK: Duration = Duration::from_millis(80);

/// Size guess (logical px) until the UI reports the real one.
const GUESS_WIDTH: f64 = 220.0;
const GUESS_ROW: f64 = 22.0;
const GUESS_SEPARATOR: f64 = 11.0;
const GUESS_PADDING: f64 = 10.0;

/// The panel's inner padding (logical px): a submenu moves up by this much so its first row lines
/// up with the row that opened it.
const PANEL_PADDING: f64 = 5.0;
/// Menus keep this far (logical px) from the left, right and bottom screen edges, like macOS.
const EDGE_MARGIN: f64 = 10.0;
/// The submenu overlaps its parent by this much (logical px), like macOS.
const SUBMENU_OVERLAP: f64 = 2.0;

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

    /// The mouse moved onto another bar item while a menu is open: switch menus, no click.
    /// Never opens a menu on its own and never closes one.
    fn hover(&mut self, key: &str) -> Action {
        match &self.open {
            Some(open) if open != key => {
                self.open = Some(key.to_string());
                self.blurred_at = None;
                Action::Swap(key.to_string())
            }
            _ => Action::Nothing,
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

/// The open submenu (window `menu-sub`).
struct Sub {
    index: usize,
    /// Logical px from the menu window's top to the row that opened it.
    trigger_top: f64,
    /// Last size the UI reported (logical px).
    size: (f64, f64),
    /// The UI reported a real size, so the window can be placed.
    reported: bool,
    /// The window is up.
    visible: bool,
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
    /// Where the main window sits on screen (physical px).
    root_rect: Rect,
    /// Last size the UI reported (logical px).
    size: (f64, f64),
    rev: u64,
    /// The window is placed but not shown yet: it waits for the UI to report its size, so the
    /// first thing on screen is the right panel and not a stale frame.
    pending_show: bool,
    /// Bumped on every open and close; a delayed hide only counts if nothing happened since.
    generation: u64,
    sub: Option<Sub>,
    acrylic: bool,
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

/// Makes one hidden menu window. `no_activate` keeps the submenu from ever taking focus, so the
/// keyboard stays in the main menu.
fn build_window(
    app: &AppHandle,
    label: &str,
    no_activate: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("Barista menu")
        .visible(false)
        .focused(false)
        .decorations(false)
        .transparent(true)
        // The soft shadow comes from Windows (DWM), so it matches the round corners.
        .shadow(true)
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
    #[allow(unused_mut)]
    let mut acrylic = false;
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        acrylic = crate::platform::windows::window::set_menu_styles(hwnd, no_activate);
    }
    #[cfg(not(windows))]
    let _ = no_activate;
    crate::window_radius::register(app, label, crate::window_radius::Mode::Normal)?;
    Ok(acrylic)
}

/// Makes the two hidden menu windows. Call once from `setup`.
pub fn init(app: &AppHandle, registry: Arc<Registry>) -> Result<(), Box<dyn std::error::Error>> {
    let acrylic = build_window(app, LABEL, false)?;
    build_window(app, SUB_LABEL, true)?;
    if let Some(window) = app.get_webview_window(LABEL) {
        window.on_window_event(|event| {
            if let tauri::WindowEvent::Focused(focused) = event {
                if let Some(menu) = get() {
                    menu.focus_changed(*focused);
                }
            }
        });
    }
    let _ = MENU.set(MenuWindow {
        app: app.clone(),
        registry,
        inner: Mutex::new(Inner {
            machine: Machine::default(),
            shown: None,
            bounds: HashMap::new(),
            item_rect: Rect::default(),
            root_rect: Rect::default(),
            size: (0.0, 0.0),
            rev: 0,
            pending_show: false,
            generation: 0,
            sub: None,
            acrylic,
        }),
    });
    tone_changed(crate::appearance::current().tone == crate::appearance::luminance::Tone::Dark);
    Ok(())
}

/// The bar's tone changed: the menus follow it (dark tone = dark acrylic and light text).
pub fn tone_changed(dark: bool) {
    #[cfg(windows)]
    if let Some(menu) = get() {
        for label in [LABEL, SUB_LABEL] {
            if let Some(hwnd) = menu.app.get_webview_window(label).and_then(|w| w.hwnd().ok()) {
                crate::platform::windows::window::set_menu_dark(hwnd, dark);
            }
        }
    }
    #[cfg(not(windows))]
    let _ = dark;
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

/// Rough size of a menu (logical px), so the window can be placed before the UI measures it.
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

/// The area menus may use: the work area, pulled in from the left, right and bottom edges. The
/// top stays where it is (the menu hangs from the bar). A screen too small for it is left alone.
fn inset_work(work: &Rect, scale: f64) -> Rect {
    let m = (EDGE_MARGIN * scale).round() as i32;
    if work.width() <= 2 * m + 1 || work.height() <= m + 1 {
        return *work;
    }
    Rect {
        left: work.left + m,
        top: work.top,
        right: work.right - m,
        bottom: work.bottom - m,
    }
}

/// Logical size -> physical, then `anchor::place`. The window is one panel, so the "root width"
/// is the whole width and the side is always the right one.
/// `extra` is the window's rim (visible frame minus page, see `frame_extra`), so the page ends up
/// exactly `size` big.
fn place_logical(item: &Rect, size: (f64, f64), extra: (i32, i32), work: &Rect, scale: f64) -> Rect {
    let px = |v: f64| (v * scale).round() as i32;
    let (w, h) = (px(size.0) + extra.0, px(size.1) + extra.1);
    anchor::place(item, (w, h), w, &inset_work(work, scale)).0
}

/// The thin rim Windows keeps between the page and the visible window edge (0 if unknown).
fn frame_extra(window: Option<&tauri::WebviewWindow>) -> (i32, i32) {
    #[cfg(windows)]
    if let Some(m) = window
        .and_then(|w| w.hwnd().ok())
        .and_then(crate::platform::windows::window::frame_metrics)
    {
        return m.extra;
    }
    #[cfg(not(windows))]
    let _ = window;
    (0, 0)
}

/// Puts the window so its VISIBLE frame is exactly `rect`. A window with a shadow carries an
/// invisible border; ignoring it pushed menus up to 10 px past the screen edge.
fn move_to(window: &tauri::WebviewWindow, rect: &Rect) {
    #[cfg(windows)]
    if let Some((hwnd, m)) = window
        .hwnd()
        .ok()
        .and_then(|h| crate::platform::windows::window::frame_metrics(h).map(|m| (h, m)))
    {
        crate::platform::windows::window::set_outer_rect(
            hwnd,
            rect.left - m.inset.0,
            rect.top - m.inset.1,
            rect.width() + m.inset.0 + m.inset.2,
            rect.height() + m.inset.1 + m.inset.3,
        );
        return;
    }
    let _ = window.set_size(PhysicalSize::new(rect.width() as u32, rect.height() as u32));
    let _ = window.set_position(PhysicalPosition::new(rect.left, rect.top));
}

/// Shows a window without taking focus (the submenu must never steal the keyboard).
fn show_without_focus(window: &tauri::WebviewWindow) {
    transitions(window, false);
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        crate::platform::windows::window::set_visible(hwnd, true);
        // Same acrylic as the main menu: DWM flattens it on an inactive window otherwise.
        crate::platform::windows::window::appear_active(hwnd);
        return;
    }
    let _ = window.show();
}

/// Hides the submenu window. It is shown with a raw Win32 call (above), so Tauri's own state
/// still says "hidden" and `window.hide()` would do nothing: it must be hidden the same way.
fn hide_without_focus(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        crate::platform::windows::window::set_visible(hwnd, false);
        return;
    }
    let _ = window.hide();
}

/// Turns Windows' show/hide animation on or off for one window (no-op off Windows).
fn transitions(window: &tauri::WebviewWindow, enabled: bool) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        crate::platform::windows::window::set_menu_transitions(hwnd, enabled);
    }
    #[cfg(not(windows))]
    let _ = (window, enabled);
}

impl MenuWindow {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn window(&self) -> Option<tauri::WebviewWindow> {
        self.app.get_webview_window(LABEL)
    }

    fn sub_window(&self) -> Option<tauri::WebviewWindow> {
        self.app.get_webview_window(SUB_LABEL)
    }

    fn item_click(&self, key: &str) {
        let action = self.lock().machine.click(key, Instant::now());
        self.apply(action);
    }

    /// The mouse moved onto a bar item (`menuHover`). Switches menus if one is open.
    pub fn hover_item(&self, key: &str) {
        if !self.registry.opens_menu(key, MouseButton::Left) {
            return;
        }
        let action = self.lock().machine.hover(key);
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
            Action::Open(key) => self.show(&key, false),
            Action::Swap(key) => self.show(&key, true),
            // The machine already forgot which key; the event names the last one we showed.
            Action::Close => {
                let key = self.lock().shown.clone().unwrap_or_default();
                self.hide(&key);
            }
        }
    }

    /// Places the window and tells the UI what to draw. A fresh open stays hidden until the UI
    /// reports its size (see `pending_show`); a swap keeps the window up.
    fn show(&self, key: &str, swap: bool) {
        // The item vanished between the click and now, or the bar has no monitor yet: close
        // whatever is showing, so the window and the state machine agree.
        let (Some(snapshot), Some((bar, work, scale))) = (self.registry.menu(key), geometry())
        else {
            self.close();
            return;
        };
        self.close_submenu();
        let extra = frame_extra(self.window().as_ref());
        let (w, h) = guess_size(&snapshot);
        let (rect, item, acrylic, generation, pending) = {
            let mut inner = self.lock();
            #[cfg(windows)]
            let cursor = crate::platform::windows::window::cursor_x();
            #[cfg(not(windows))]
            let cursor: Option<i32> = None;
            inner.item_rect = match inner.bounds.get(key) {
                Some(b) => anchor::to_screen(b, &bar, scale),
                None => anchor::at_cursor(cursor.unwrap_or(bar.left), &bar),
            };
            inner.size = (w, h);
            inner.rev = snapshot.rev;
            inner.shown = Some(key.to_string());
            // A swap before the first reveal must not cancel it: still waiting = still pending.
            inner.pending_show = !swap || inner.pending_show;
            inner.generation += 1;
            let rect = place_logical(&inner.item_rect, inner.size, extra, &work, scale);
            inner.root_rect = rect;
            (rect, inner.item_rect, inner.acrylic, inner.generation, inner.pending_show)
        };
        if let Some(window) = self.window() {
            move_to(&window, &rect);
        }
        let _ = MenuOpen {
            item_id: key.to_string(),
            menu: snapshot.entries,
            anchor: MenuAnchor {
                item,
                monitor_id: crate::controller::get()
                    .map(|c| c.info().monitor.id)
                    .unwrap_or_default(),
                scale,
                acrylic,
            },
        }
        .emit(&self.app);
        if pending {
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(REVEAL_FALLBACK).await;
                if let Some(menu) = get() {
                    let go = {
                        let inner = menu.lock();
                        inner.generation == generation && inner.pending_show
                    };
                    if go {
                        menu.reveal();
                    }
                }
            });
        }
    }

    /// Shows the main window and gives it the keyboard, so arrows, Enter and Esc work at once.
    fn reveal(&self) {
        {
            let mut inner = self.lock();
            if inner.shown.is_none() {
                return;
            }
            inner.pending_show = false;
        }
        if let Some(window) = self.window() {
            transitions(&window, false);
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    /// Closes now: the UI is told, and Windows hides both windows with its own fade-out (glass
    /// and shadow can't be faded from CSS, so a CSS fade left a blank ghost panel behind).
    fn hide(&self, key: &str) {
        if key.is_empty() {
            return;
        }
        {
            let mut inner = self.lock();
            inner.shown = None;
            inner.pending_show = false;
            inner.sub = None;
            inner.generation += 1;
        }
        let _ = MenuClose {
            item_id: key.to_string(),
        }
        .emit(&self.app);
        if let Some(window) = self.window() {
            transitions(&window, true);
            let _ = window.hide();
        }
        if let Some(window) = self.sub_window() {
            transitions(&window, true);
            hide_without_focus(&window);
        }
    }

    /// `menuResized`: the UI measured the main panel (logical px). Re-places the window and, the
    /// first time after an open, shows it.
    pub fn resized(&self, width: f64, height: f64) {
        let Some((_, work, scale)) = geometry() else {
            return;
        };
        let extra = frame_extra(self.window().as_ref());
        let (rect, reveal) = {
            let mut inner = self.lock();
            if inner.shown.is_none() {
                return;
            }
            inner.size = (width, height);
            let rect = place_logical(&inner.item_rect, inner.size, extra, &work, scale);
            inner.root_rect = rect;
            (rect, inner.pending_show)
        };
        if let Some(window) = self.window() {
            move_to(&window, &rect);
        }
        if reveal {
            self.reveal();
        }
        self.place_submenu();
    }

    /// `openSubmenu`: the entry `index` of the open menu was hovered or opened by keyboard.
    /// `trigger_top` is the row's top in logical px, relative to the menu window.
    pub fn open_submenu(&self, index: usize, trigger_top: f64) {
        let Some(key) = self.lock().shown.clone() else {
            return;
        };
        let Some(snapshot) = self.registry.menu(&key) else {
            return;
        };
        let Some(entry) = snapshot
            .entries
            .get(index)
            .filter(|e| e.kind == MenuItemType::Submenu && e.enabled)
        else {
            return self.close_submenu();
        };
        {
            let mut inner = self.lock();
            if matches!(&inner.sub, Some(s) if s.index == index && s.visible) {
                return;
            }
            let visible = inner.sub.as_ref().is_some_and(|s| s.visible);
            inner.sub = Some(Sub {
                index,
                trigger_top,
                size: (GUESS_WIDTH, GUESS_ROW),
                reported: false,
                visible,
            });
        }
        let _ = SubmenuShow {
            item_id: key,
            index: index as u32,
            entries: entry.submenu.clone(),
        }
        .emit(&self.app);
    }

    /// `submenuResized`: the submenu window's UI measured its panel (logical px). Places and
    /// shows it.
    pub fn submenu_resized(&self, width: f64, height: f64) {
        {
            let mut inner = self.lock();
            let Some(sub) = inner.sub.as_mut() else {
                return;
            };
            sub.size = (width, height);
            sub.reported = true;
        }
        self.place_submenu();
    }

    /// Puts the submenu window next to the main one, from the sizes we know.
    fn place_submenu(&self) {
        let Some((_, work, scale)) = geometry() else {
            return;
        };
        let px = |v: f64| (v * scale).round() as i32;
        let extra = frame_extra(self.sub_window().as_ref());
        let (rect, side, root) = {
            let mut inner = self.lock();
            let root = inner.root_rect;
            let Some(sub) = inner.sub.as_mut() else {
                return;
            };
            if !sub.reported {
                return;
            }
            sub.visible = true;
            let trigger = root.top + px(sub.trigger_top - PANEL_PADDING);
            let (rect, side) = anchor::place_submenu(
                &root,
                trigger,
                (px(sub.size.0) + extra.0, px(sub.size.1) + extra.1),
                px(SUBMENU_OVERLAP),
                &inset_work(&work, scale),
            );
            (rect, side, root)
        };
        if let Some(window) = self.sub_window() {
            move_to(&window, &rect);
            show_without_focus(&window);
            // The submenu may have been closed after we read the state but before the window
            // showed (the pointer left the row). Showing it then would leave an empty window
            // that nothing tracks, so hide it again.
            if self.lock().sub.is_none() {
                hide_without_focus(&window);
                return;
            }
        }
        let _ = SubmenuPlaced {
            side,
            x: f64::from(rect.left - root.left) / scale,
            y: f64::from(rect.top - root.top) / scale,
            width: f64::from(rect.width()) / scale,
            height: f64::from(rect.height()) / scale,
        }
        .emit(&self.app);
    }

    /// `closeSubmenu`: hide the submenu now (no fade: it closes while the pointer moves on).
    pub fn close_submenu(&self) {
        let (had, key) = {
            let mut inner = self.lock();
            (inner.sub.take().is_some(), inner.shown.clone())
        };
        // Always hide, even if no submenu was tracked: hiding a hidden window is free, and it
        // clears any window a racing show left behind.
        if let Some(window) = self.sub_window() {
            hide_without_focus(&window);
        }
        if !had {
            return;
        }
        let _ = SubmenuHide {
            item_id: key.unwrap_or_default(),
        }
        .emit(&self.app);
    }

    /// Mouse over a submenu row: the main menu's UI follows it.
    pub fn submenu_hover(&self, index: Option<u32>) {
        let _ = SubmenuHovered { index }.emit(&self.app);
    }

    /// Keyboard moved the submenu's highlight: the submenu's UI follows it.
    pub fn set_submenu_highlight(&self, index: Option<u32>) {
        let _ = SubmenuHighlight { index }.emit(&self.app);
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
        let (key, item, acrylic) = {
            let inner = self.lock();
            (inner.shown.clone()?, inner.item_rect, inner.acrylic)
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
                acrylic,
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
                // The submenu may no longer exist: the UI opens it again if it still does.
                self.close_submenu();
                let _ = MenuChanged {
                    item_id: key,
                    menu: snapshot.entries,
                }
                .emit(&self.app);
            }
        }
    }
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
    fn hovering_another_icon_swaps_only_while_a_menu_is_open() {
        let mut m = Machine::default();
        let t = Instant::now();
        // Nothing open: hovering never opens a menu.
        assert_eq!(m.hover("s1/a"), Action::Nothing);
        m.click("s1/a", t);
        // Same icon: nothing. Another icon: swap.
        assert_eq!(m.hover("s1/a"), Action::Nothing);
        assert_eq!(m.hover("s2/b"), Action::Swap("s2/b".into()));
        assert_eq!(m.open.as_deref(), Some("s2/b"));
        // Hovering never closes: a click on the open icon still does.
        assert_eq!(m.click("s2/b", t), Action::Close);
        assert_eq!(m.hover("s1/a"), Action::Nothing);
    }

    #[test]
    fn hover_swap_cancels_a_pending_blur() {
        let mut m = Machine::default();
        let t = Instant::now();
        m.click("s1/a", t);
        m.blur(t);
        assert_eq!(m.hover("s2/b"), Action::Swap("s2/b".into()));
        assert_eq!(m.blur_timeout(later(t, 200)), Action::Nothing);
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
    fn menus_keep_a_margin_from_the_screen_edges_but_not_from_the_bar() {
        let work = Rect { left: 0, top: 36, right: 1920, bottom: 1080 };
        let inset = inset_work(&work, 1.5);
        assert_eq!((inset.left, inset.top, inset.right, inset.bottom), (15, 36, 1905, 1065));
        // A menu that wants to sit at the right edge now stops short of it.
        let item = Rect { left: 1900, top: 0, right: 1910, bottom: 36 };
        let placed = place_logical(&item, (200.0, 100.0), (4, 4), &work, 1.5);
        assert_eq!(placed.right, 1905);
        // The page itself is 200 logical px wide: the rim comes on top.
        assert_eq!(placed.width(), 300 + 4);
        // A tiny screen is left alone.
        let tiny = Rect { left: 0, top: 0, right: 10, bottom: 10 };
        assert_eq!(inset_work(&tiny, 1.0).right, 10);
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
