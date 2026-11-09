//! Monitor list, work areas and DPI, read straight from Windows.

use crate::bar::{Monitor, Rect};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, DISPLAY_DEVICEW, HDC, HMONITOR,
    MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

/// `MONITORINFOF_PRIMARY`: set in `MONITORINFO.dwFlags` for the primary monitor.
const MONITORINFOF_PRIMARY: u32 = 1;
/// `EDD_GET_DEVICE_INTERFACE_NAME`: makes EnumDisplayDevicesW return the device path.
const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 1;

fn to_rect(r: RECT) -> Rect {
    Rect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

/// Turns a fixed-size UTF-16 buffer (zero terminated) into a String.
fn utf16_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// Called by Windows once per monitor. It can't capture variables, so the Vec comes in as
/// a raw pointer hidden in `data` (the usual Win32 callback trick).
unsafe extern "system" fn collect(
    handle: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> windows::core::BOOL {
    // SAFETY: `data` is the `&mut Vec` we passed to EnumDisplayMonitors below, which is
    // alive for the whole (synchronous) enumeration.
    let out = unsafe { &mut *(data.0 as *mut Vec<Monitor>) };
    if let Some(monitor) = read_monitor(handle) {
        out.push(monitor);
    }
    TRUE
}

fn read_monitor(handle: HMONITOR) -> Option<Monitor> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: MONITORINFOEXW starts with a MONITORINFO, and cbSize tells Windows which of the
    // two structs it got.
    let ok = unsafe { GetMonitorInfoW(handle, &mut info as *mut _ as *mut MONITORINFO) };
    if !ok.as_bool() {
        return None;
    }

    let device = utf16_to_string(&info.szDevice); // e.g. \\.\DISPLAY1
    let mut dpi_x = 96;
    let mut dpi_y = 96;
    // Failure (rare) keeps 96 = 100%.
    // SAFETY: plain out-pointers to local variables.
    let _ = unsafe { GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };

    Some(Monitor {
        id: stable_id(&info.szDevice).unwrap_or_else(|| device.clone()),
        name: device,
        is_primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        rect: to_rect(info.monitorInfo.rcMonitor),
        work_area: to_rect(info.monitorInfo.rcWork),
        scale: f64::from(dpi_x) / 96.0,
    })
}

/// `\\.\DISPLAY2` can change when a monitor is replugged. The device path of the display
/// itself (`\\?\DISPLAY#GSM5B09#...`) stays the same, so that is our id.
fn stable_id(device_name: &[u16; 32]) -> Option<String> {
    let mut dd = DISPLAY_DEVICEW {
        cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
        ..Default::default()
    };
    // SAFETY: device_name is zero terminated (it came from Windows); dd is a valid out struct.
    let ok = unsafe {
        EnumDisplayDevicesW(
            PCWSTR(device_name.as_ptr()),
            0,
            &mut dd,
            EDD_GET_DEVICE_INTERFACE_NAME,
        )
    };
    if !ok.as_bool() {
        return None;
    }
    let id = utf16_to_string(&dd.DeviceID);
    (!id.is_empty()).then_some(id)
}

/// All connected monitors, in the order Windows lists them.
pub fn list() -> Vec<Monitor> {
    let mut monitors: Vec<Monitor> = Vec::new();
    // SAFETY: the pointer in LPARAM is only used inside `collect`, during this call.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut monitors as *mut Vec<Monitor> as isize),
        );
    }
    monitors
}
