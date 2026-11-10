//! Where a menu window goes. Pure math in physical pixels, so every edge case has a test.
//!
//! ```text
//!  bar   ...  [icon]  ...                    |  <- work area right edge
//!             +--------+-------+             |
//!             | root   | sub   |   fits: submenu on the right
//!             +--------+-------+             |
//!                              [icon]        |
//!                     +-------+--------+     |
//!                     | sub   | root   |  <- too wide: root shifts left to fit,
//!                     +-------+--------+     submenu flips to the left
//! ```

use crate::bar::Rect;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Where an item sits inside the bar window, in logical pixels (CSS pixels, what the UI
/// measures with `getBoundingClientRect`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ItemBounds {
    /// The item's `id` from `getItems()` / `itemsChanged`.
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Which side an open submenu goes on, so it stays on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum SubmenuSide {
    Right,
    Left,
}

/// Logical, bar-relative bounds -> physical screen pixels. The one place this conversion
/// happens for menus (CLAUDE.md rule 8).
pub fn to_screen(b: &ItemBounds, bar: &Rect, scale: f64) -> Rect {
    let px = |v: f64| (v * scale).round() as i32;
    Rect {
        left: bar.left + px(b.x),
        top: bar.top + px(b.y),
        right: bar.left + px(b.x + b.width),
        bottom: bar.top + px(b.y + b.height),
    }
}

/// Without bounds from the UI: a 1 px wide item under the mouse, as tall as the bar.
pub fn at_cursor(cursor_x: i32, bar: &Rect) -> Rect {
    let x = cursor_x.clamp(bar.left, bar.right - 1);
    Rect {
        left: x,
        top: bar.top,
        right: x + 1,
        bottom: bar.bottom,
    }
}

/// Places a menu of `size` (width, height; physical) under `item`, inside `work`.
/// `root_width` is the main panel's width; anything wider is an open submenu.
/// - The root panel starts at the item's left edge, shifted left if it would leave the screen.
/// - The submenu goes right if it fits, else left (the root panel stays where it was).
/// - Height is capped to the work area (the UI scrolls a very long menu).
pub fn place(item: &Rect, size: (i32, i32), root_width: i32, work: &Rect) -> (Rect, SubmenuSide) {
    let width = size.0.clamp(1, work.width().max(1));
    let root_width = root_width.clamp(1, width);
    let extra = width - root_width;
    let top = item.bottom.clamp(work.top, work.bottom - 1);
    let height = size.1.clamp(1, (work.bottom - top).max(1));

    let root_left = item.left.min(work.right - root_width).max(work.left);
    let (left, side) = if root_left + width <= work.right {
        (root_left, SubmenuSide::Right)
    } else {
        ((root_left - extra).max(work.left), SubmenuSide::Left)
    };
    let rect = Rect {
        left,
        top,
        right: left + width,
        bottom: top + height,
    };
    (rect, side)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    /// A monitor at `origin_x` with a bar of 28 logical px at `scale`.
    fn monitor(origin_x: i32, width: i32, scale: f64) -> (Rect, Rect) {
        let bar_h = (28.0 * scale).round() as i32;
        let bar = rect(origin_x, 0, origin_x + width, bar_h);
        let work = rect(origin_x, bar_h, origin_x + width, (1080.0 * scale) as i32);
        (bar, work)
    }

    fn icon_at(x: f64) -> ItemBounds {
        ItemBounds {
            id: "s1/a".into(),
            x,
            y: 2.0,
            width: 24.0,
            height: 24.0,
        }
    }

    #[test]
    fn menu_opens_under_the_icon_at_every_scale() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let (bar, work) = monitor(0, (1920.0 * scale) as i32, scale);
            let item = to_screen(&icon_at(400.0), &bar, scale);
            let size = ((220.0 * scale) as i32, (180.0 * scale) as i32);
            let (r, side) = place(&item, size, size.0, &work);
            assert_eq!(r.left, (400.0 * scale).round() as i32, "scale {scale}");
            assert_eq!(r.top, bar.bottom, "touches the bar, scale {scale}");
            assert_eq!((r.width(), r.height()), size);
            assert_eq!(side, SubmenuSide::Right);
        }
    }

    #[test]
    fn near_the_right_edge_the_menu_shifts_left() {
        let (bar, work) = monitor(0, 1920, 1.0);
        let item = to_screen(&icon_at(1890.0), &bar, 1.0);
        let (r, _) = place(&item, (220, 100), 220, &work);
        assert_eq!(r.right, 1920);
        assert_eq!(r.left, 1700);
    }

    #[test]
    fn near_the_left_edge_the_menu_stays_on_screen() {
        let (bar, work) = monitor(0, 1920, 1.0);
        let (r, _) = place(&at_cursor(-50, &bar), (220, 100), 220, &work);
        assert_eq!(r.left, 0);
    }

    #[test]
    fn submenu_flips_left_when_it_does_not_fit() {
        let (bar, work) = monitor(0, 1920, 1.5);
        let item = to_screen(&icon_at(1100.0), &bar, 1.5); // left edge at 1650 px
                                                           // Root alone fits (1650 + 220 = 1870), root + submenu (440) does not.
        let (r, side) = place(&item, (440, 300), 220, &work);
        assert_eq!(side, SubmenuSide::Left);
        assert_eq!(r.right, 1650 + 220, "root panel did not move");
        // With room, it opens to the right.
        let item = to_screen(&icon_at(100.0), &bar, 1.5);
        assert_eq!(place(&item, (440, 300), 220, &work).1, SubmenuSide::Right);
    }

    #[test]
    fn second_monitor_with_negative_coordinates() {
        // A monitor left of the primary: x runs from -2560 to 0.
        let (bar, work) = monitor(-2560, 2560, 1.25);
        let item = to_screen(&icon_at(10.0), &bar, 1.25);
        assert_eq!(item.left, -2560 + 13);
        let (r, _) = place(&item, (300, 200), 300, &work);
        assert_eq!(r.left, -2547);
        let item = to_screen(&icon_at(2040.0), &bar, 1.25); // 2550 px in: near its right edge
        let (r, _) = place(&item, (300, 200), 300, &work);
        assert_eq!(r.right, 0);
    }

    #[test]
    fn a_very_tall_menu_is_capped_to_the_work_area() {
        let (bar, work) = monitor(0, 1920, 1.0);
        let item = to_screen(&icon_at(10.0), &bar, 1.0);
        let (r, _) = place(&item, (220, 5000), 220, &work);
        assert_eq!(r.bottom, work.bottom);
    }

    #[test]
    fn cursor_fallback_is_clamped_to_the_bar() {
        let (bar, _) = monitor(0, 1920, 1.0);
        assert_eq!(at_cursor(5000, &bar).left, 1919);
        assert_eq!(at_cursor(300, &bar).bottom, bar.bottom);
    }
}
