//! Which part of the wallpaper image sits under the bar? Windows places the image on each monitor
//! in one of six ways ("Fill", "Fit", ...). We work out where the image lands, cut the bar's strip
//! (top `strip_h` rows of the monitor) out of that, and map it back to image pixels. Pure maths.
//!
//! Everything is in physical pixels (CLAUDE.md rule 8).

/// How Windows places the wallpaper (the values of `DESKTOP_WALLPAPER_POSITION`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Center,
    Tile,
    Stretch,
    Fit,
    Fill,
    Span,
}

impl Placement {
    pub fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Center,
            1 => Self::Tile,
            2 => Self::Stretch,
            3 => Self::Fit,
            4 => Self::Fill,
            5 => Self::Span,
            _ => Self::Fill,
        }
    }
}

/// A rectangle in image pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crop {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// What the bar's strip shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripPlan {
    /// The part of the image under the strip, if any.
    pub crop: Option<Crop>,
    /// Share of the strip (0.0..=1.0) that is NOT image: the plain background colour shows there
    /// (letterbox bars of "Fit", the border of "Center").
    pub background_share: f64,
}

/// `mon_w` x `mon_h` is the monitor, `strip_h` the bar's height, `img_w` x `img_h` the image.
pub fn plan_strip(
    placement: Placement,
    mon_w: f64,
    mon_h: f64,
    strip_h: f64,
    img_w: f64,
    img_h: f64,
) -> StripPlan {
    if mon_w <= 0.0 || mon_h <= 0.0 || strip_h <= 0.0 || img_w <= 0.0 || img_h <= 0.0 {
        return StripPlan {
            crop: None,
            background_share: 1.0,
        };
    }
    let strip_h = strip_h.min(mon_h);

    // Where the whole image lands on the monitor (x, y, w, h), maybe bigger than the monitor.
    let (dx, dy, dw, dh) = match placement {
        // Cover the monitor, crop the overflow. "Span" is treated the same (we do not stitch
        // the monitors together; the bar is a thin strip, so the error is small).
        Placement::Fill | Placement::Span => {
            let s = (mon_w / img_w).max(mon_h / img_h);
            let (w, h) = (img_w * s, img_h * s);
            ((mon_w - w) / 2.0, (mon_h - h) / 2.0, w, h)
        }
        Placement::Fit => {
            let s = (mon_w / img_w).min(mon_h / img_h);
            let (w, h) = (img_w * s, img_h * s);
            ((mon_w - w) / 2.0, (mon_h - h) / 2.0, w, h)
        }
        Placement::Stretch => (0.0, 0.0, mon_w, mon_h),
        Placement::Center => ((mon_w - img_w) / 2.0, (mon_h - img_h) / 2.0, img_w, img_h),
        // Tiles start at the monitor's top-left corner. Looking at the first tile is close
        // enough: a tile repeats the same pixels.
        Placement::Tile => (0.0, 0.0, img_w.min(mon_w), img_h.min(strip_h.max(0.0))),
    };

    if placement == Placement::Tile {
        return StripPlan {
            crop: Some(Crop {
                x: 0.0,
                y: 0.0,
                w: dw,
                h: dh,
            }),
            background_share: 0.0,
        };
    }

    // The strip is (0, 0, mon_w, strip_h). Cut it with the image's rectangle.
    let left = dx.max(0.0);
    let top = dy.max(0.0);
    let right = (dx + dw).min(mon_w);
    let bottom = (dy + dh).min(strip_h);
    if right <= left || bottom <= top {
        return StripPlan {
            crop: None,
            background_share: 1.0,
        };
    }
    let covered = (right - left) * (bottom - top);
    let background_share = (1.0 - covered / (mon_w * strip_h)).clamp(0.0, 1.0);

    // Back to image pixels.
    let (sx, sy) = (img_w / dw, img_h / dh);
    StripPlan {
        crop: Some(Crop {
            x: (left - dx) * sx,
            y: (top - dy) * sy,
            w: (right - left) * sx,
            h: (bottom - top) * sy,
        }),
        background_share,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn fill_same_ratio_takes_the_top_rows() {
        // 4K image on a 4K monitor, 48 px bar: the top 48 rows, full width.
        let p = plan_strip(Placement::Fill, 3840.0, 2160.0, 48.0, 3840.0, 2160.0);
        let c = p.crop.unwrap();
        assert!(close(c.x, 0.0) && close(c.y, 0.0) && close(c.w, 3840.0) && close(c.h, 48.0));
        assert!(close(p.background_share, 0.0));
    }

    #[test]
    fn fill_crops_the_sides_of_a_wide_image_and_scales_the_strip() {
        // Image twice as wide (2000x500) on a 1000x500 monitor: scale 1, sides cut, 40 rows.
        let p = plan_strip(Placement::Fill, 1000.0, 500.0, 40.0, 2000.0, 500.0);
        let c = p.crop.unwrap();
        assert!(close(c.x, 500.0) && close(c.w, 1000.0) && close(c.h, 40.0));
    }

    #[test]
    fn fill_crops_the_top_of_a_tall_image() {
        // 1000x2000 image on 1000x1000 monitor: scale 1, vertical overflow 500 each side.
        let p = plan_strip(Placement::Fill, 1000.0, 1000.0, 50.0, 1000.0, 2000.0);
        let c = p.crop.unwrap();
        assert!(close(c.y, 500.0) && close(c.h, 50.0));
    }

    #[test]
    fn fit_letterbox_is_background() {
        // Square image on a wide monitor: fits by height, so the strip is full image width of
        // the middle part, with bars left and right.
        let p = plan_strip(Placement::Fit, 2000.0, 1000.0, 50.0, 1000.0, 1000.0);
        assert!(close(p.background_share, 0.5));
        let c = p.crop.unwrap();
        assert!(close(c.w, 1000.0) && close(c.h, 50.0));
    }

    #[test]
    fn fit_with_bars_on_top_is_all_background() {
        // Very wide image on a tall monitor: the strip sits in the top letterbox bar.
        let p = plan_strip(Placement::Fit, 1000.0, 1000.0, 50.0, 2000.0, 500.0);
        assert!(p.crop.is_none());
        assert!(close(p.background_share, 1.0));
    }

    #[test]
    fn stretch_scales_each_axis() {
        // 100x100 image stretched to 1000x500: a 50 px strip is 10 image rows.
        let p = plan_strip(Placement::Stretch, 1000.0, 500.0, 50.0, 100.0, 100.0);
        let c = p.crop.unwrap();
        assert!(close(c.w, 100.0) && close(c.h, 10.0) && close(c.y, 0.0));
    }

    #[test]
    fn center_small_image_leaves_a_background_border() {
        // 500x500 image centred on 1000x1000: it starts 250 px down, so a 50 px strip misses it.
        let p = plan_strip(Placement::Center, 1000.0, 1000.0, 50.0, 500.0, 500.0);
        assert!(p.crop.is_none());
        // Same image on a 1000x500 monitor still starts at y = 0.
        let p = plan_strip(Placement::Center, 1000.0, 500.0, 50.0, 500.0, 500.0);
        assert!(close(p.background_share, 0.5));
    }

    #[test]
    fn center_big_image_takes_the_middle() {
        // 3000x3000 image centred on 1000x1000 monitor: strip is at image y 1000.
        let p = plan_strip(Placement::Center, 1000.0, 1000.0, 50.0, 3000.0, 3000.0);
        let c = p.crop.unwrap();
        assert!(close(c.x, 1000.0) && close(c.y, 1000.0) && close(c.w, 1000.0) && close(c.h, 50.0));
    }

    #[test]
    fn tile_uses_the_first_tile() {
        let p = plan_strip(Placement::Tile, 1000.0, 500.0, 40.0, 200.0, 200.0);
        let c = p.crop.unwrap();
        assert!(close(c.w, 200.0) && close(c.h, 40.0));
        assert!(close(p.background_share, 0.0));
    }

    #[test]
    fn bad_sizes_give_background_only() {
        let p = plan_strip(Placement::Fill, 0.0, 500.0, 40.0, 200.0, 200.0);
        assert!(p.crop.is_none() && close(p.background_share, 1.0));
    }

    #[test]
    fn unknown_raw_value_falls_back_to_fill() {
        assert_eq!(Placement::from_raw(99), Placement::Fill);
        assert_eq!(Placement::from_raw(3), Placement::Fit);
    }
}
