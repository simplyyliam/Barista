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
use crate::events::PanelChanged;
use crate::placement::{self, geometry};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

pub const LABEL: &str = "panel";

/// If the UI never reports its size (it should within a frame), show the window anyway.
const REVEAL_FALLBACK: Duration = Duration::from_millis(30);

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
        }),
    });
    crate::menu_window::tone_changed(
        crate::appearance::current().tone == crate::appearance::luminance::Tone::Dark,
    );
    Ok(())
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
    pub fn open(&self, name: &str, anchor: &str, size: (f64, f64)) {
        if !valid_name(name) || !valid_anchor(anchor) {
            return;
        }
        let action = self.lock().machine.click(anchor, Instant::now());
        match action {
            Action::Nothing => {}
            Action::Open(_) | Action::Swap(_) => self.show(name, anchor, size),
            Action::Close => self.hide(),
        }
    }

    fn show(&self, name: &str, anchor: &str, size: (f64, f64)) {
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

        let extra = crate::menu_window::frame_extra(self.window().as_ref());
        let item = placement::item_rect(anchor, &bar, scale);
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
                acrylic: inner.acrylic,
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
        let extra = crate::menu_window::frame_extra(self.window().as_ref());
        let (rect, reveal) = {
            let mut inner = self.lock();
            let Some(open) = inner.open.as_mut() else {
                return;
            };
            let item = placement::item_rect(&open.anchor, &bar, scale);
            let rect = crate::menu_window::place_logical(&item, (width, height), extra, &work, scale);
            open.rect = rect;
            open.scale = scale;
            inner.size = (width, height);
            (rect, inner.pending_show)
        };
        if let Some(window) = self.window() {
            crate::menu_window::move_to(&window, &rect);
        }
        if reveal {
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
