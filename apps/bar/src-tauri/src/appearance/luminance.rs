//! How bright is a strip of pixels, and should text on it be dark or light? Pure maths, no Windows.
//!
//! In plain words:
//! 1. Screens store colours "bent" (sRGB gamma) so dark shades get more numbers. Brightness adds
//!    up in straight light, so we first un-bend each channel (`channel_to_linear`).
//! 2. Our eyes are most sensitive to green, then red, then blue. The WCAG formula weighs them
//!    that way: `L = 0.2126 R + 0.7152 G + 0.0722 B`. L is 0 for black and 1 for white.
//! 3. We average L over every pixel (not the colours first: a half black, half white strip
//!    should read as "mixed", not as one grey).
//! 4. Contrast ratio between text and background is `(L1 + 0.05) / (L2 + 0.05)`, lighter on top.
//!    Black text (L = 0) has contrast `(L + 0.05) / 0.05`, white text (L = 1) has `1.05 / (L + 0.05)`.
//!    They are equal at `L = sqrt(1.05 * 0.05) - 0.05`, about 0.179. Brighter than that: black text
//!    reads better, so the wallpaper is `Light`. Darker: white text, so `Dark`.

use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

/// How bright the wallpaper behind the bar is. Light wallpaper = dark content, and the other way round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Light,
    Dark,
}

/// Luminance where black and white text have the same contrast: `sqrt(1.05 * 0.05) - 0.05`.
const THRESHOLD: f64 = 0.179;

/// Un-bends one sRGB channel (0..=255) into straight light (0.0..=1.0). WCAG 2.x formula.
fn channel_to_linear(value: u8) -> f64 {
    let c = f64::from(value) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Relative luminance of one colour (WCAG).
pub fn relative_luminance(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * channel_to_linear(r) + 0.7152 * channel_to_linear(g) + 0.0722 * channel_to_linear(b)
}

/// Average luminance of 4-byte pixels in B, G, R, x order (what WIC's 32bppBGR gives).
/// `None` for an empty buffer.
pub fn average_luminance(bgrx: &[u8]) -> Option<f64> {
    // 256 entries, built once: far cheaper than a `powf` per channel on a 4K strip.
    static TABLE: OnceLock<[f64; 256]> = OnceLock::new();
    let table = TABLE.get_or_init(|| std::array::from_fn(|i| channel_to_linear(i as u8)));
    let mut sum = 0.0;
    let mut count = 0u64;
    for px in bgrx.chunks_exact(4) {
        sum += 0.0722 * table[px[0] as usize]
            + 0.7152 * table[px[1] as usize]
            + 0.2126 * table[px[2] as usize];
        count += 1;
    }
    (count > 0).then(|| sum / count as f64)
}

pub fn tone_for(luminance: f64) -> Tone {
    if luminance > THRESHOLD {
        Tone::Light
    } else {
        Tone::Dark
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_and_white_are_the_ends() {
        assert_eq!(relative_luminance(0, 0, 0), 0.0);
        assert!((relative_luminance(255, 255, 255) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn green_counts_more_than_blue() {
        assert!(relative_luminance(0, 255, 0) > relative_luminance(0, 0, 255) * 5.0);
    }

    #[test]
    fn middle_grey_is_much_darker_than_half() {
        // sRGB 128 is about 21% light, not 50%: that is the gamma at work.
        let l = relative_luminance(128, 128, 128);
        assert!((0.2..0.22).contains(&l), "{l}");
    }

    #[test]
    fn tone_follows_the_threshold() {
        assert_eq!(tone_for(1.0), Tone::Light);
        assert_eq!(tone_for(0.0), Tone::Dark);
        assert_eq!(tone_for(0.18), Tone::Light);
        assert_eq!(tone_for(0.178), Tone::Dark);
    }

    #[test]
    fn pure_blue_is_dark_and_yellow_is_light() {
        assert_eq!(tone_for(relative_luminance(0, 0, 255)), Tone::Dark);
        assert_eq!(tone_for(relative_luminance(255, 255, 0)), Tone::Light);
    }

    #[test]
    fn threshold_matches_the_formula() {
        let exact = (1.05f64 * 0.05).sqrt() - 0.05;
        assert!((exact - THRESHOLD).abs() < 0.001, "{exact}");
    }

    #[test]
    fn average_is_taken_over_light_not_over_colour() {
        // One white pixel and one black pixel: average light is 0.5 (grey 188 would be ~0.5).
        let px = [255, 255, 255, 0, 0, 0, 0, 0];
        let avg = average_luminance(&px).unwrap();
        assert!((avg - 0.5).abs() < 1e-9);
        assert_eq!(average_luminance(&[]), None);
    }
}
