//! Reads the desktop wallpaper under the bar and measures how bright it is.
//!
//! We read the wallpaper FILE instead of capturing the screen. A capture would also contain the
//! bar itself and any window dragged over it, and making the bar invisible to captures would
//! hide it from the user's own screenshots too.
//!
//! Flow: `IDesktopWallpaper` (Windows' own wallpaper API) tells us the image path, how it is
//! placed and the background colour for our monitor. WIC (Windows' built-in image decoder)
//! reads only the strip the bar covers, shrunk to at most 512 px wide, so even a huge 8K
//! wallpaper costs a few milliseconds and a small buffer.
//!
//! COM must be started on the thread that calls `strip_luminance` (the appearance worker does).

use crate::appearance::layout::{plan_strip, Placement};
use crate::appearance::luminance::{average_luminance, relative_luminance};
use crate::bar::Rect;
use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::{GENERIC_READ, RECT};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGR, IWICBitmapSource, IWICImagingFactory,
    IWICPalette, WICBitmapDitherTypeNone, WICBitmapInterpolationModeFant,
    WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnDemand, WICRect,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{DesktopWallpaper, IDesktopWallpaper};

/// Widest strip we decode. The average of a 512 px strip is as good as the average of 8000 px.
const MAX_SAMPLE_WIDTH: u32 = 512;

/// Starts COM for the calling thread. Call once, from the thread that will sample.
pub fn init_com() {
    // SAFETY: plain COM start-up on a thread we own; a repeat call just returns S_FALSE.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
    }
}

/// Average luminance (0..=1) of the strip the bar covers: the top `strip_h` physical pixels of
/// `monitor`. `None` when it cannot be worked out (the caller keeps the last answer).
pub fn strip_luminance(monitor: Rect, strip_h: i32) -> Option<f64> {
    // SAFETY: COM calls on an initialised thread. Every pointer comes from Windows and is used
    // only while the owning interface is alive in this function.
    unsafe { strip_luminance_inner(monitor, strip_h).ok().flatten() }
}

unsafe fn strip_luminance_inner(monitor: Rect, strip_h: i32) -> windows::core::Result<Option<f64>> {
    let wallpaper: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)?;

    let Some(id) = monitor_id(&wallpaper, monitor)? else {
        return Ok(None);
    };
    let path = wallpaper.GetWallpaper(PCWSTR(id.as_ptr()))?;
    let path_text = pwstr_to_string(path);
    CoTaskMemFree(Some(path.0 as *const _));

    let colour = wallpaper.GetBackgroundColor()?.0; // 0x00BBGGRR
    let background = relative_luminance(
        (colour & 0xFF) as u8,
        ((colour >> 8) & 0xFF) as u8,
        ((colour >> 16) & 0xFF) as u8,
    );
    // No image (solid colour wallpaper): the background colour is the whole answer.
    if path_text.is_empty() {
        return Ok(Some(background));
    }

    let placement = Placement::from_raw(wallpaper.GetPosition()?.0);
    let plan_for = |img_w: f64, img_h: f64| {
        plan_strip(
            placement,
            f64::from(monitor.width()),
            f64::from(monitor.height()),
            f64::from(strip_h),
            img_w,
            img_h,
        )
    };

    // Windows keeps a converted copy of the current wallpaper here; it works when the original
    // file is gone (a deleted file, a theme, Spotlight). It only holds the wallpaper of the monitor
    // at the desktop origin, so another monitor must not borrow it (it could show another image).
    let fallback = (monitor.left == 0 && monitor.top == 0)
        .then(|| std::env::var("APPDATA").ok())
        .flatten()
        .map(|a| format!("{a}\\Microsoft\\Windows\\Themes\\TranscodedWallpaper"));
    for candidate in std::iter::once(path_text).chain(fallback) {
        if let Some((image, share)) = image_luminance(&candidate, &plan_for) {
            return Ok(Some(image * (1.0 - share) + background * share));
        }
    }
    Ok(None)
}

/// The wallpaper API names monitors by device path; find ours by comparing rectangles.
unsafe fn monitor_id(
    w: &IDesktopWallpaper,
    monitor: Rect,
) -> windows::core::Result<Option<Vec<u16>>> {
    for i in 0..w.GetMonitorDevicePathCount()? {
        let path = w.GetMonitorDevicePathAt(i)?;
        let mut id: Vec<u16> = path.as_wide().to_vec();
        id.push(0);
        CoTaskMemFree(Some(path.0 as *const _));
        // A monitor that was unplugged can still be listed but has no rectangle: skip it.
        let Ok(r) = w.GetMonitorRECT(PCWSTR(id.as_ptr())) else {
            continue;
        };
        if same_rect(r, monitor) {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

fn same_rect(a: RECT, b: Rect) -> bool {
    a.left == b.left && a.top == b.top && a.right == b.right && a.bottom == b.bottom
}

unsafe fn pwstr_to_string(p: windows::core::PWSTR) -> String {
    if p.is_null() {
        String::new()
    } else {
        String::from_utf16_lossy(p.as_wide())
    }
}

/// Average luminance of the strip in the image at `path`, plus the share of the strip that is
/// not image (see `StripPlan`). `None` if the file cannot be read.
unsafe fn image_luminance(
    path: &str,
    plan_for: &dyn Fn(f64, f64) -> crate::appearance::layout::StripPlan,
) -> Option<(f64, f64)> {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let factory: IWICImagingFactory =
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_ALL).ok()?;
    let decoder = factory
        .CreateDecoderFromFilename(
            PCWSTR(wide.as_ptr()),
            None,
            GENERIC_READ,
            WICDecodeMetadataCacheOnDemand,
        )
        .ok()?;
    let frame = decoder.GetFrame(0).ok()?;
    let (mut img_w, mut img_h) = (0u32, 0u32);
    frame.GetSize(&mut img_w, &mut img_h).ok()?;

    let plan = plan_for(f64::from(img_w), f64::from(img_h));
    let Some(crop) = plan.crop else {
        // The strip sits entirely on the background colour. Say so with a zero-size image part.
        return Some((0.0, 1.0));
    };

    // Whole pixels, at least 1x1, inside the image.
    let x = (crop.x.floor() as i32).clamp(0, img_w as i32 - 1);
    let y = (crop.y.floor() as i32).clamp(0, img_h as i32 - 1);
    let w = (crop.w.ceil() as i32).clamp(1, img_w as i32 - x);
    let h = (crop.h.ceil() as i32).clamp(1, img_h as i32 - y);

    let clipper = factory.CreateBitmapClipper().ok()?;
    clipper
        .Initialize(
            &frame,
            &WICRect {
                X: x,
                Y: y,
                Width: w,
                Height: h,
            },
        )
        .ok()?;
    let mut source: IWICBitmapSource = clipper.cast().ok()?;

    if w as u32 > MAX_SAMPLE_WIDTH {
        let new_h = ((h as u32 * MAX_SAMPLE_WIDTH) / w as u32).max(1);
        let scaler = factory.CreateBitmapScaler().ok()?;
        scaler
            .Initialize(
                &source,
                MAX_SAMPLE_WIDTH,
                new_h,
                WICBitmapInterpolationModeFant,
            )
            .ok()?;
        source = scaler.cast().ok()?;
    }

    // Always read as B, G, R, unused, whatever the file's own format is.
    let converter = factory.CreateFormatConverter().ok()?;
    converter
        .Initialize(
            &source,
            &GUID_WICPixelFormat32bppBGR,
            WICBitmapDitherTypeNone,
            None::<&IWICPalette>,
            0.0,
            WICBitmapPaletteTypeCustom,
        )
        .ok()?;
    let (mut out_w, mut out_h) = (0u32, 0u32);
    converter.GetSize(&mut out_w, &mut out_h).ok()?;
    let mut pixels = vec![0u8; out_w as usize * out_h as usize * 4];
    converter
        .CopyPixels(std::ptr::null(), out_w * 4, &mut pixels)
        .ok()?;

    Some((average_luminance(&pixels)?, plan.background_share))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads (never changes) the real wallpaper under the primary monitor's bar and prints the
    /// result and the time: `cargo test wallpaper_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn wallpaper_live() {
        init_com();
        // The real app is per-monitor DPI aware (Tauri's manifest); a test process is not, and
        // would see scaled monitor rectangles that never match the wallpaper API's.
        // SAFETY: called once, before any window exists in this process.
        unsafe {
            let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
                windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            );
        }
        let monitor = crate::platform::windows::monitors::list()
            .into_iter()
            .find(|m| m.is_primary)
            .expect("a primary monitor");
        let started = std::time::Instant::now();
        let l = strip_luminance(monitor.rect, 48).expect("a measurement");
        eprintln!(
            "luminance {l:.3} -> {:?} in {:?}",
            crate::appearance::luminance::tone_for(l),
            started.elapsed()
        );
        assert!((0.0..=1.0).contains(&l));
    }
}
