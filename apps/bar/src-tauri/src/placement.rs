//! Where things go on screen: the bar strip, the usable work area, and the rect of a bar item.
//! Shared by menus and popovers. Everything here is physical pixels.

use crate::bar::Rect;
use crate::registry::anchor::{self, ItemBounds};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Menus keep this far (logical px) from the left, right and bottom screen edges, like macOS.
const EDGE_MARGIN: f64 = 10.0;

fn bounds() -> &'static Mutex<HashMap<String, ItemBounds>> {
    static BOUNDS: OnceLock<Mutex<HashMap<String, ItemBounds>>> = OnceLock::new();
    BOUNDS.get_or_init(Default::default)
}

/// `setItemBounds`: where the UI drew each item (logical, relative to the bar window), keyed like
/// `s3/counter`. Converted to screen pixels only when something opens, with the bar's position and
/// scale at that moment (it may have moved monitor since).
pub fn set_item_bounds(items: Vec<ItemBounds>) {
    *bounds().lock().unwrap_or_else(|e| e.into_inner()) = items
        .into_iter()
        .take(256) // the UI is trusted, but a runaway loop shouldn't grow this forever
        .map(|b| (b.id.clone(), b))
        .collect();
}

/// The item's rect on screen. Without bounds from the UI, a slim rect under the mouse x.
pub fn item_rect(key: &str, bar: &Rect, scale: f64) -> Rect {
    let found = bounds().lock().unwrap_or_else(|e| e.into_inner()).get(key).cloned();
    match found {
        Some(b) => anchor::to_screen(&b, bar, scale),
        None => {
            #[cfg(windows)]
            let cursor = crate::platform::windows::window::cursor_x();
            #[cfg(not(windows))]
            let cursor: Option<i32> = None;
            anchor::at_cursor(cursor.unwrap_or(bar.left), bar)
        }
    }
}

/// The bar's strip, the area menus may use, and the scale, from the controller.
pub fn geometry() -> Option<(Rect, Rect, f64)> {
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
pub fn inset_work(work: &Rect, scale: f64) -> Rect {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_margin_from_the_screen_edges_but_not_from_the_bar() {
        let work = Rect { left: 0, top: 36, right: 1920, bottom: 1080 };
        let inset = inset_work(&work, 1.5);
        assert_eq!((inset.left, inset.top, inset.right, inset.bottom), (15, 36, 1905, 1065));
        // A tiny screen is left alone.
        let tiny = Rect { left: 0, top: 0, right: 10, bottom: 10 };
        assert_eq!(inset_work(&tiny, 1.0).right, 10);
    }
}
