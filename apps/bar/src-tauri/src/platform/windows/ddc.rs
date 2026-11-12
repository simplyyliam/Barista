//! External monitor brightness over DDC/CI (the `dxva2` monitor configuration API). It talks to the
//! monitor over its cable, so a call can take hundreds of milliseconds and some monitors (or docks)
//! do not answer at all: callers run it on a thread that may wait, and a monitor that fails or is
//! very slow is reported as unsupported.

use std::time::{Duration, Instant};
use windows::Win32::Devices::Display::{
    DestroyPhysicalMonitors, GetMonitorBrightness, GetNumberOfPhysicalMonitorsFromHMONITOR,
    GetPhysicalMonitorsFromHMONITOR, SetMonitorBrightness, PHYSICAL_MONITOR,
};
use windows::Win32::Foundation::{LPARAM, RECT, TRUE};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, DISPLAY_DEVICEW, HDC, HMONITOR,
    MONITORINFOEXW,
};

/// Separates the parts of a device id (`MONITOR\BOE0A1B\{guid}\0001`).
pub const PATH_SEP: char = '\\';

/// A read slower than this counts as "too slow": the slider would lag, so the UI gets no control.
pub const SLOW: Duration = Duration::from_millis(1500);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawDdc {
    /// `ddc:<display device>:<index>`, stable while the monitors stay plugged in.
    pub id: String,
    pub name: String,
    /// 0 to 100, `None` when the monitor did not answer or answered too slowly.
    pub percent: Option<u8>,
}

fn utf16_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

unsafe extern "system" fn collect(
    handle: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> windows::core::BOOL {
    // SAFETY: `data` is the `&mut Vec` passed to EnumDisplayMonitors below, alive for the call.
    let list = unsafe { &mut *(data.0 as *mut Vec<HMONITOR>) };
    list.push(handle);
    TRUE
}

fn monitors() -> Vec<(HMONITOR, String)> {
    let mut handles: Vec<HMONITOR> = Vec::new();
    // SAFETY: the callback only touches the Vec, which outlives the call.
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(collect), LPARAM(&mut handles as *mut _ as isize));
    }
    handles
        .into_iter()
        .map(|h| {
            let mut info = MONITORINFOEXW::default();
            info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
            // SAFETY: `info` has the size Windows is told.
            let ok = unsafe { GetMonitorInfoW(h, (&mut info as *mut MONITORINFOEXW).cast()) }.as_bool();
            let device = if ok { utf16_to_string(&info.szDevice) } else { String::new() };
            (h, device)
        })
        .collect()
}

/// The PnP id of the monitor on a display device (`BOE0A1B` from `MONITOR\BOE0A1B\{guid}\0001`).
fn pnp_id(device: &str) -> Option<String> {
    let mut wide: Vec<u16> = device.encode_utf16().collect();
    wide.push(0);
    let mut dd = DISPLAY_DEVICEW {
        cb: size_of::<DISPLAY_DEVICEW>() as u32,
        ..Default::default()
    };
    // SAFETY: `wide` is zero terminated and `dd` has the size Windows is told.
    let found = unsafe { EnumDisplayDevicesW(PCWSTR(wide.as_ptr()), 0, &mut dd, 0) }.as_bool();
    let id = utf16_to_string(&dd.DeviceID);
    found.then(|| id.split(PATH_SEP).nth(1).map(str::to_owned)).flatten()
}

/// The physical monitors behind one HMONITOR. Destroy them with `release`.
fn physical(h: HMONITOR) -> Vec<PHYSICAL_MONITOR> {
    let mut count = 0u32;
    // SAFETY: plain calls; the array has the size Windows reported.
    unsafe {
        if GetNumberOfPhysicalMonitorsFromHMONITOR(h, &mut count).is_err() || count == 0 {
            return Vec::new();
        }
        let mut list = vec![PHYSICAL_MONITOR::default(); count as usize];
        if GetPhysicalMonitorsFromHMONITOR(h, &mut list).is_err() {
            return Vec::new();
        }
        list
    }
}

fn release(list: &[PHYSICAL_MONITOR]) {
    // SAFETY: these handles came from `physical`.
    unsafe {
        let _ = DestroyPhysicalMonitors(list);
    }
}

/// (min, current, max) as the monitor reports them.
fn range(p: &PHYSICAL_MONITOR) -> Option<(u32, u32, u32)> {
    let (mut min, mut cur, mut max) = (0, 0, 0);
    // SAFETY: plain call with live out values.
    let ok = unsafe { GetMonitorBrightness(p.hPhysicalMonitor, &mut min, &mut cur, &mut max) };
    (ok != 0 && max > min).then_some((min, cur.clamp(min, max), max))
}

/// (min, max) per monitor id, filled by `read_all`: a slider drag then needs one cable write per
/// change instead of a read and a write.
static RANGES: std::sync::Mutex<Vec<(String, u32, u32)>> = std::sync::Mutex::new(Vec::new());

fn remember(id: &str, min: u32, max: u32) {
    let mut r = RANGES.lock().unwrap_or_else(|e| e.into_inner());
    match r.iter_mut().find(|(i, _, _)| i == id) {
        Some(e) => (e.1, e.2) = (min, max),
        None => r.push((id.to_owned(), min, max)),
    }
}

fn remembered(id: &str) -> Option<(u32, u32)> {
    let r = RANGES.lock().unwrap_or_else(|e| e.into_inner());
    r.iter().find(|(i, _, _)| i == id).map(|e| (e.1, e.2))
}

/// Maps the monitor's own range to 0..100.
pub fn to_percent(min: u32, cur: u32, max: u32) -> u8 {
    (u64::from(cur.saturating_sub(min)) * 100 / u64::from((max - min).max(1))).min(100) as u8
}

/// Maps 0..100 back to the monitor's own range.
pub fn from_percent(min: u32, max: u32, percent: u8) -> u32 {
    min + (u64::from(max - min) * u64::from(percent.min(100)) / 100) as u32
}

/// Asks every monitor except the one with PnP id `skip` (the laptop panel, which WMI handles).
/// Slow when a monitor is slow: call from a worker thread.
pub fn read_all(skip: Option<&str>) -> Vec<RawDdc> {
    let mut out = Vec::new();
    for (h, device) in monitors() {
        if skip.is_some_and(|s| !s.is_empty() && pnp_id(&device).as_deref() == Some(s)) {
            continue;
        }
        let list = physical(h);
        for (i, p) in list.iter().enumerate() {
            let started = Instant::now();
            let id = format!("ddc:{device}:{i}");
            let percent = range(p)
                .filter(|_| started.elapsed() < SLOW)
                .map(|(min, cur, max)| {
                    remember(&id, min, max);
                    to_percent(min, cur, max)
                });
            // The struct is packed: copy the array out before borrowing it.
            let description = p.szPhysicalMonitorDescription;
            out.push(RawDdc {
                id,
                name: utf16_to_string(&description),
                percent,
            });
        }
        release(&list);
    }
    out
}

/// Sets one monitor (`id` from `read_all`). Returns whether the monitor took it.
pub fn set(id: &str, percent: u8) -> bool {
    for (h, device) in monitors() {
        let list = physical(h);
        let mut done = false;
        for (i, p) in list.iter().enumerate() {
            if format!("ddc:{device}:{i}") == id {
                let known = remembered(id).or_else(|| range(p).map(|(min, _, max)| (min, max)));
                if let Some((min, max)) = known {
                    // SAFETY: plain call on a live handle.
                    done = unsafe { SetMonitorBrightness(p.hPhysicalMonitor, from_percent(min, max, percent)) } != 0;
                }
            }
        }
        release(&list);
        if done {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_map_both_ways() {
        assert_eq!(to_percent(0, 50, 100), 50);
        assert_eq!(to_percent(10, 10, 90), 0);
        assert_eq!(to_percent(10, 90, 90), 100);
        assert_eq!(from_percent(0, 100, 30), 30);
        assert_eq!(from_percent(10, 90, 50), 50);
        assert_eq!(from_percent(10, 90, 200), 90);
    }

    /// Writes the value it just read, so the monitor does not change.
    #[test]
    #[ignore = "needs a DDC/CI monitor"]
    fn sets_a_monitor_to_itself() {
        let m = read_all(None).into_iter().find(|m| m.percent.is_some()).expect("a DDC monitor");
        let p = m.percent.unwrap();
        let ok = set(&m.id, p);
        println!("set {} to {p}: {ok}, last error {:?}", m.id, windows::core::Error::from_thread());
        assert!(ok);
    }

    #[test]
    #[ignore = "needs a desktop session"]
    fn lists_monitors() {
        for m in read_all(None) {
            println!("{m:?}");
        }
    }
}
