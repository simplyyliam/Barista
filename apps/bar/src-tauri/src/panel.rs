//! The panel window: one hidden, transparent window made at startup and reused for every system
//! panel (control center, notifications, battery, ...). The content is a route the UI picks from
//! the panel's `name`; Rust decides when it opens, where it goes and when it closes (ADR 0016).
//! Same ideas as the menu window (ADR 0010): the UI measures itself, Rust places.
//!
//! ```text
//!  UI: openPanel(name, anchor, w, h) ─► Machine (toggle same anchor / swap another)
//!      open: close menu + popover, place under the anchor, emit panelChanged
//!  UI renders the route, measures ─► panelResized ─► place again, show + focus (first time)
//!  close: closePanel (Esc, the UI decides) / same anchor again / focus lost (after a grace)
//!         / a menu, popover or other item is clicked (`close()` from `route_click`)
//! ```

use crate::bar::Rect;
use crate::dismiss::{Action, Machine, BLUR_GRACE};
use crate::events::{PanelChanged, PanelWarm};
use crate::placement::{self, geometry};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

pub const LABEL: &str = "panel";

/// If the UI never reports its size (it should within a frame), show the window anyway.
const REVEAL_FALLBACK: Duration = Duration::from_millis(150);

/// How the panel window looks. `Glass` is the system backdrop, rounded corners and shadow of a menu.
/// `None` is a bare transparent window: the page draws its own cards (the notification and control
/// center designs are separate floating cards).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PanelChrome {
    #[default]
    Glass,
    None,
}

/// A panel and the item it hangs from. The UI shows the route for `name` and the item as open.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PanelOpen {
    pub name: String,
    pub anchor: String,
    /// Where the window sits on screen (physical pixels).
    pub rect: Rect,
    pub scale: f64,
    pub monitor_id: String,
    /// True when the window has the system acrylic backdrop. False (Windows 10): draw an opaque panel.
    pub acrylic: bool,
    pub chrome: PanelChrome,
}

struct Inner {
    machine: Machine,
    open: Option<PanelOpen>,
    /// Last size the UI reported (logical px).
    size: (f64, f64),
    /// Placed but not shown yet: waits for the UI to report its size.
    pending_show: bool,
    /// Bumped on every open and close; a delayed reveal only counts if nothing happened since.
    generation: u64,
    acrylic: bool,
    chrome: PanelChrome,
    /// The last measured size of each route (logical px). A new open places the window at this size
    /// at once, so it is not resized around the open (a resize makes WebView2 paint the top first and
    /// the bottom a beat later).
    sizes: HashMap<String, (f64, f64)>,
    /// Startup warm-up: the route whose size the page may report while nothing is open.
    warming: Option<&'static str>,
}

pub struct PanelWindow {
    app: AppHandle,
    inner: Mutex<Inner>,
}

static PANEL: OnceLock<PanelWindow> = OnceLock::new();

pub fn get() -> Option<&'static PanelWindow> {
    PANEL.get()
}

/// Closes the panel if one is open. Safe to call any time (a menu or popover is opening).
pub fn close() {
    if let Some(p) = get() {
        p.close();
    }
}

/// Another window got the foreground (the shared hook in `popover.rs` calls this).
pub fn foreground_changed(pid: u32) {
    if let Some(p) = get() {
        p.foreground_changed(pid);
    }
}

/// Panel names are route names: short, lowercase, digits and dashes.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Anchors are item ids like `s3/counter` or `system/battery`.
fn valid_anchor(anchor: &str) -> bool {
    !anchor.is_empty() && anchor.len() <= 256
}

/// Makes the hidden panel window. Call once from `setup`, after the menu window.
pub fn init(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let acrylic = crate::menu_window::build_window(app, LABEL, false)?;
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.set_title("Barista panel");
        window.on_window_event(|event| {
            if let tauri::WindowEvent::Focused(focused) = event {
                if let Some(p) = get() {
                    p.focus_changed(*focused);
                }
            }
        });
    }
    let _ = PANEL.set(PanelWindow {
        app: app.clone(),
        inner: Mutex::new(Inner {
            machine: Machine::default(),
            open: None,
            size: (0.0, 0.0),
            pending_show: false,
            generation: 0,
            acrylic,
            chrome: PanelChrome::Glass,
            sizes: HashMap::new(),
            warming: None,
        }),
    });
    crate::menu_window::tone_changed(
        crate::appearance::current().tone == crate::appearance::luminance::Tone::Dark,
    );
    warm_up(app);
    Ok(())
}

/// The very first show of a window costs about 130 ms more than the next ones (Windows and WebView2
/// build their surfaces then), which is what the first click on the clock would feel like. Pay it
/// at startup instead: show the window cloaked (invisible) and without taking focus, then hide it.
fn warm_up(app: &AppHandle) {
    #[cfg(windows)]
    {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            // Let the page load and the bar settle first.
            tokio::time::sleep(Duration::from_secs(3)).await;
            if get().and_then(|p| p.current()).is_some() {
                return; // already opened by the user: nothing to warm
            }
            let Some(raw) = app
                .get_webview_window(LABEL)
                .and_then(|w| w.hwnd().ok())
                .map(|h| h.0 as isize)
            else {
                return;
            };
            // Let the page draw its content (the notification pictures are the expensive part to
            // paint) and report its real size (`resized` sizes the hidden window), then show it
            // cloaked. Window calls belong on the thread that owns the window.
            if let Some(p) = get() {
                p.lock().warming = Some("notifications");
            }
            let _ = PanelWarm(true).emit(&app);
            tokio::time::sleep(Duration::from_millis(300)).await;
            let _ = app.run_on_main_thread(move || {
                crate::platform::windows::window::warm_show(windows::Win32::Foundation::HWND(
                    raw as *mut _,
                ));
            });
            tokio::time::sleep(Duration::from_millis(500)).await;
            let _ = PanelWarm(false).emit(&app);
            if let Some(p) = get() {
                p.lock().warming = None;
            }
            let _ = app.run_on_main_thread(move || {
                // If the user opened the panel in the meantime, leave it showing.
                let keep = get().and_then(|p| p.current()).is_some();
                crate::platform::windows::window::warm_end(
                    windows::Win32::Foundation::HWND(raw as *mut _),
                    keep,
                );
            });
        });
    }
    #[cfg(not(windows))]
    let _ = app;
}

/// What the panel hangs from. A glass panel hangs from the item itself (left edges line up, right
/// under the bar, like a menu). A bare-window panel is centred under the item and sits a little
/// below the bar (`--spacing-panel-offset`, 14 px in App.png).
fn hang_rect(item: Rect, size: (f64, f64), extra: (i32, i32), scale: f64, chrome: PanelChrome) -> Rect {
    if chrome == PanelChrome::Glass {
        return item;
    }
    let width = (size.0 * scale).round() as i32 + extra.0;
    let left = (item.left + item.right) / 2 - width / 2;
    Rect {
        left,
        right: left + 1,
        top: item.top,
        bottom: item.bottom + (14.0 * scale).round() as i32,
    }
}

/// Switches the window between the menu look and a bare transparent window.
fn apply_chrome(window: &tauri::WebviewWindow, chrome: PanelChrome) {
    let glass = chrome == PanelChrome::Glass;
    let _ = window.set_shadow(glass);
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        crate::platform::windows::window::set_panel_chrome(hwnd, glass);
    }
}

impl PanelWindow {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn window(&self) -> Option<tauri::WebviewWindow> {
        self.app.get_webview_window(LABEL)
    }

    /// The panel that is open, for `getOpenPanel`.
    pub fn current(&self) -> Option<PanelOpen> {
        self.lock().open.clone()
    }

    /// `openPanel`: a click on the item `anchor`. Same anchor again closes, another anchor swaps.
    /// `size` is the UI's first guess (logical px); `panelResized` fixes it.
    pub fn open(&self, name: &str, anchor: &str, size: (f64, f64), chrome: PanelChrome) {
        if !valid_name(name) || !valid_anchor(anchor) {
            return;
        }
        let action = self.lock().machine.click(anchor, Instant::now());
        match action {
            Action::Nothing => {}
            Action::Open(_) | Action::Swap(_) => self.show(name, anchor, size, chrome),
            Action::Close => self.hide(),
        }
    }

    fn show(&self, name: &str, anchor: &str, size: (f64, f64), chrome: PanelChrome) {
        let Some((bar, work, scale)) = geometry() else {
            // The bar has no monitor yet: stay closed, and keep the machine honest.
            self.close();
            return;
        };
        // One thing open at a time in the whole bar.
        if let Some(menu) = crate::menu_window::get() {
            menu.close();
        }
        crate::popover::close();
        crate::system::panel_opened(name);
        // Place the window at the size this route had last time (the UI's number is only a first
        // guess), so the open does not resize it.
        let size = self.lock().sizes.get(name).copied().unwrap_or(size);

        // Windows sends no event when the battery estimate drifts or a DST change slips by:
        // read both fresh whenever a panel opens (one cheap call each, no polling).
        crate::system::battery::refresh();
        crate::system::clock::refresh();
        // The folder watcher is quick for new toasts but Windows signals removals late (seconds to
        // never), so opening the notification panel checks for that. Only a real change reads again:
        // otherwise the list would re-layout right after the window shows.
        if name == "notifications" {
            crate::system::notifications::refresh_if_changed();
        }
        // The frame decides the invisible border, so set the chrome before measuring it.
        if self.lock().chrome != chrome {
            if let Some(window) = self.window() {
                apply_chrome(&window, chrome);
            }
            self.lock().chrome = chrome;
        }
        let extra = crate::menu_window::frame_extra(self.window().as_ref());
        let item = placement::item_rect(anchor, &bar, scale);
        let item = hang_rect(item, size, extra, scale, chrome);
        let (open, generation) = {
            let mut inner = self.lock();
            inner.size = size;
            inner.generation += 1;
            // A swap before the first reveal must not cancel it: still waiting = still pending.
            inner.pending_show = inner.open.is_none() || inner.pending_show;
            let rect = crate::menu_window::place_logical(&item, size, extra, &work, scale);
            let open = PanelOpen {
                name: name.to_string(),
                anchor: anchor.to_string(),
                rect,
                scale,
                monitor_id: crate::controller::get()
                    .map(|c| c.info().monitor.id)
                    .unwrap_or_default(),
                acrylic: inner.acrylic && chrome == PanelChrome::Glass,
                chrome,
            };
            inner.open = Some(open.clone());
            (open, inner.generation)
        };
        if let Some(window) = self.window() {
            crate::menu_window::move_to(&window, &open.rect);
        }
        let _ = PanelChanged {
            open: Some(open),
        }
        .emit(&self.app);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(REVEAL_FALLBACK).await;
            if let Some(p) = get() {
                let go = {
                    let inner = p.lock();
                    inner.generation == generation && inner.pending_show
                };
                if go {
                    p.reveal();
                }
            }
        });
    }

    /// `panelResized`: the UI measured its content (logical px). Places the window again and, the
    /// first time after an open, shows it.
    pub fn resized(&self, width: f64, height: f64) {
        let Some((bar, work, scale)) = geometry() else {
            return;
        };
        // Startup warm-up: nothing is open, but the page measured its content. Remember that size and
        // put the hidden window at it, so the first real open needs no resize.
        let warming = {
            let inner = self.lock();
            if inner.open.is_none() { inner.warming } else { None }
        };
        if let Some(name) = warming {
            self.lock().sizes.insert(name.to_string(), (width, height));
            if let Some(window) = self.window() {
                let _ = window.set_size(tauri::LogicalSize::new(width, height));
            }
            return;
        }
        let extra = crate::menu_window::frame_extra(self.window().as_ref());
        let (rect, reveal, changed, size_changed, generation) = {
            let mut inner = self.lock();
            let Some(open) = inner.open.as_mut() else {
                return;
            };
            let item = placement::item_rect(&open.anchor, &bar, scale);
            let item = hang_rect(item, (width, height), extra, scale, open.chrome);
            let rect = crate::menu_window::place_logical(&item, (width, height), extra, &work, scale);
            open.rect = rect;
            open.scale = scale;
            let name = open.name.clone();
            let changed = open.clone();
            // `inner.size` is what the window was placed at; if the page measured something else
            // (fewer notifications, say), the window is about to be resized.
            let size_changed = inner.size != (width, height);
            inner.size = (width, height);
            inner.sizes.insert(name, (width, height));
            (rect, inner.pending_show, changed, size_changed, inner.generation)
        };
        if let Some(window) = self.window() {
            crate::menu_window::move_to(&window, &rect);
        }
        // The UI reads `rect` and `scale` from this event: keep them true after the re-place.
        let _ = PanelChanged { open: Some(changed) }.emit(&self.app);
        if !reveal {
            return;
        }
        if size_changed {
            // The window was just resized: give the page's surface two frames to catch up, or the
            // first frame shows only the old height and the rest follows a beat later.
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(40)).await;
                if let Some(p) = get() {
                    let go = {
                        let inner = p.lock();
                        inner.generation == generation && inner.pending_show
                    };
                    if go {
                        p.reveal();
                    }
                }
            });
        } else {
            self.reveal();
        }
    }

    /// Shows the window and gives it the keyboard.
    fn reveal(&self) {
        {
            let mut inner = self.lock();
            if inner.open.is_none() {
                return;
            }
            inner.pending_show = false;
        }
        if let Some(window) = self.window() {
            // Off while it opens: the panel appears whole and instantly (Windows does not animate a bare
            // transparent window anyway), and on again when it hides.
            crate::menu_window::transitions(&window, false);
            let _ = window.show();
            let _ = window.set_focus();
        }
    }

    fn focus_changed(&self, focused: bool) {
        if focused {
            self.lock().machine.focus();
            return;
        }
        self.lock().machine.blur(Instant::now());
        self.close_after_grace();
    }

    /// A program other than the bar took the foreground. Covers the case where the panel never got
    /// focus itself (Windows refused it), so no `Focused(false)` will ever come.
    pub fn foreground_changed(&self, pid: u32) {
        {
            let mut inner = self.lock();
            if inner.open.is_none() {
                return;
            }
            if pid == std::process::id() {
                inner.machine.focus();
                return;
            }
            inner.machine.blur(Instant::now());
        }
        self.close_after_grace();
    }

    /// Hides the panel after the grace unless a click on an icon or the focus decided first.
    fn close_after_grace(&self) {
        tauri::async_runtime::spawn(async {
            tokio::time::sleep(BLUR_GRACE).await;
            if let Some(p) = get() {
                let action = p.lock().machine.blur_timeout(Instant::now());
                if action == Action::Close {
                    p.hide();
                }
            }
        });
    }

    /// `closePanel` (Esc, or the UI decided) and the one-open rule. Does nothing if none is open.
    pub fn close(&self) {
        self.lock().machine.reset();
        self.hide();
    }

    /// Hides the window and tells the UI. Windows fades it out itself.
    fn hide(&self) {
        let was_open = {
            let mut inner = self.lock();
            inner.pending_show = false;
            inner.generation += 1;
            inner.open.take().is_some()
        };
        if !was_open {
            return;
        }
        let _ = PanelChanged { open: None }.emit(&self.app);
        if let Some(window) = self.window() {
            crate::menu_window::transitions(&window, true);
            let _ = window.hide();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu_window::place_logical;

    fn r(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect { left, top, right, bottom }
    }

    #[test]
    fn a_bare_panel_is_centred_under_its_item_and_below_the_bar() {
        // Item 900..1100 on a bar 0..32, scale 2, panel 340 logical = 680 physical.
        let item = r(900, 0, 1100, 32);
        let hung = hang_rect(item, (340.0, 400.0), (0, 0), 2.0, PanelChrome::None);
        assert_eq!(hung.left, 1000 - 340);
        assert_eq!(hung.bottom, 32 + 28);
        // Glass keeps hanging from the item itself.
        assert_eq!(hang_rect(item, (340.0, 400.0), (0, 0), 2.0, PanelChrome::Glass), item);
    }

    #[test]
    fn names_are_route_like() {
        assert!(valid_name("control-center"));
        assert!(valid_name("battery2"));
        assert!(!valid_name(""));
        assert!(!valid_name("Battery"));
        assert!(!valid_name("a/b"));
        assert!(!valid_name("../x"));
        assert!(!valid_name(&"a".repeat(33)));
    }

    #[test]
    fn anchors_are_bounded() {
        assert!(valid_anchor("system/battery"));
        assert!(!valid_anchor(""));
        assert!(!valid_anchor(&"x".repeat(257)));
    }

    /// The panel lands under the item at every scale and keeps its margin from the screen edge.
    #[test]
    fn panel_goes_under_the_item_at_every_scale() {
        for scale in [1.0_f64, 1.25, 1.5, 2.0] {
            let bar_h = (32.0 * scale).round() as i32;
            let work = Rect { left: 0, top: bar_h, right: 1920, bottom: 1080 };
            let item = Rect { left: 400, top: 0, right: 440, bottom: bar_h };
            let r = place_logical(&item, (320.0, 400.0), (0, 0), &work, scale);
            assert_eq!(r.top, bar_h);
            assert_eq!(r.left, 400);
            assert_eq!(r.width(), (320.0 * scale).round() as i32);
        }
    }

    #[test]
    fn panel_is_pulled_in_from_the_right_edge_and_works_left_of_the_primary() {
        // An item at the far right: the panel shifts left and keeps a 10 px margin.
        let work = Rect { left: 0, top: 32, right: 1920, bottom: 1080 };
        let item = Rect { left: 1880, top: 0, right: 1920, bottom: 32 };
        let r = place_logical(&item, (320.0, 400.0), (0, 0), &work, 1.0);
        assert_eq!(r.right, 1910);
        // A monitor left of the primary has negative x.
        let work = Rect { left: -1920, top: 32, right: 0, bottom: 1080 };
        let item = Rect { left: -100, top: 0, right: -60, bottom: 32 };
        let r = place_logical(&item, (320.0, 400.0), (0, 0), &work, 1.0);
        assert_eq!(r.right, -10);
        assert!(r.left >= -1910);
    }

    #[test]
    fn panel_height_is_capped_to_the_work_area() {
        let work = Rect { left: 0, top: 32, right: 1920, bottom: 500 };
        let item = Rect { left: 10, top: 0, right: 50, bottom: 32 };
        let r = place_logical(&item, (320.0, 5000.0), (0, 0), &work, 1.0);
        assert!(r.bottom <= 490);
    }
}
