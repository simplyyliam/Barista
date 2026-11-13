//! Night light. Windows has no API for it: the Settings app keeps the on/off state in a small
//! binary value in the user's CloudStore registry key, and its service applies a change to that
//! value right away (ADR 0020). We flip the same bytes.
//!
//! The format changes between Windows builds, so `parse` only accepts the two layouts we have
//! seen on a real machine (Windows 11, build 26300) and `toggled` only produces those. Anything
//! else reports `supported: false` and the tile is greyed out; we never write bytes we don't know.

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NightLight {
    /// False when the stored value has a layout we don't know: `on` means nothing then.
    pub supported: bool,
    pub on: bool,
}

pub fn service() -> &'static Service<NightLight> {
    static S: OnceLock<Service<NightLight>> = OnceLock::new();
    S.get_or_init(|| Service::new(NightLight::default()))
}

/// Bytes 0 to 9 are the same in every sample: a header and the start of the first timestamp.
const HEADER: [u8; 10] = [0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06];
/// The first timestamp: Unix seconds as a 5 byte varint, right after the header.
const TS1_AT: usize = 10;
const TS1_LEN: usize = 5;
const MARKER: [u8; 3] = [0x2a, 0x2b, 0x0e];
const MARKER_AT: usize = 15;
/// The state byte, then `43 42 01 00`, then the part that differs between on and off.
const STATE_AT: usize = 18;
const PART_AT: usize = 23;
const OFF_STATE: u8 = 0x13;
const ON_STATE: u8 = 0x15;
/// On is the off layout with these two bytes put in front of `d0 0a 02`.
const ON_EXTRA: [u8; 2] = [0x10, 0x00];
const SCHEDULE: [u8; 3] = [0xd0, 0x0a, 0x02];
/// After that: `c6 14` and the second timestamp (a Windows FILETIME) as a 9 byte varint.
const TS2_TAG: [u8; 2] = [0xc6, 0x14];
const TS2_LEN: usize = 9;

/// Both timestamps of "now", as Windows writes them when it changes the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now {
    pub unix_secs: u64,
    pub filetime: u64,
}

impl Now {
    /// Now, but never the same second twice: Windows ignores a change whose time is not newer, so
    /// two quick clicks within one second would otherwise lose the second one.
    pub fn system() -> Self {
        static LAST: std::sync::Mutex<u64> = std::sync::Mutex::new(0);
        let real = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        let unix_secs = if real > *last { real } else { *last + 1 };
        *last = unix_secs;
        Self { unix_secs, filetime: (unix_secs + 11_644_473_600) * 10_000_000 }
    }
}

/// Writes `value` as a varint of exactly `len` bytes (more bytes than needed keep their
/// continuation bit). `None` if it does not fit.
fn put_varint(out: &mut [u8], value: u64) -> Option<()> {
    let len = out.len();
    if len < 10 && value >> (7 * len) != 0 {
        return None;
    }
    for (i, byte) in out.iter_mut().enumerate() {
        let part = ((value >> (7 * i)) & 0x7f) as u8;
        *byte = if i + 1 < len { part | 0x80 } else { part };
    }
    Some(())
}

fn is_varint(bytes: &[u8]) -> bool {
    bytes.last().is_some_and(|b| b & 0x80 == 0) && bytes[..bytes.len() - 1].iter().all(|b| b & 0x80 != 0)
}

/// Where the second timestamp starts, if the bytes after the state look like one of the layouts.
fn ts2_at(blob: &[u8], on: bool) -> Option<usize> {
    let part = PART_AT + if on { ON_EXTRA.len() } else { 0 };
    let tag = part + SCHEDULE.len();
    let at = tag + TS2_TAG.len();
    let ok = blob.len() >= at + TS2_LEN
        && blob[part..tag] == SCHEDULE
        && blob[tag..at] == TS2_TAG
        && is_varint(&blob[at..at + TS2_LEN]);
    ok.then_some(at)
}

fn layout_ok(blob: &[u8]) -> bool {
    blob.len() > PART_AT
        && blob[..HEADER.len()] == HEADER
        && blob[MARKER_AT..MARKER_AT + 3] == MARKER
        && is_varint(&blob[TS1_AT..TS1_AT + TS1_LEN])
}

/// `Some(true)` = on, `Some(false)` = off, `None` = a layout we don't know.
pub fn parse(blob: &[u8]) -> Option<bool> {
    if !layout_ok(blob) {
        return None;
    }
    match blob[STATE_AT] {
        OFF_STATE if ts2_at(blob, false).is_some() => Some(false),
        ON_STATE if blob[PART_AT..PART_AT + 2] == ON_EXTRA && ts2_at(blob, true).is_some() => Some(true),
        _ => None,
    }
}

/// The blob as Windows would write it after switching night light on or off at `now`: the state,
/// the two bytes that mark "on", and both timestamps (Windows ignores a change that doesn't look
/// newer). `None` if the layout is unknown.
pub fn with_state(blob: &[u8], on: bool, now: Now) -> Option<Vec<u8>> {
    let current = parse(blob)?;
    let mut out = blob.to_vec();
    if current != on {
        out[STATE_AT] = if on { ON_STATE } else { OFF_STATE };
        if on {
            out.splice(PART_AT..PART_AT, ON_EXTRA);
        } else {
            out.drain(PART_AT..PART_AT + ON_EXTRA.len());
        }
    }
    put_varint(&mut out[TS1_AT..TS1_AT + TS1_LEN], now.unix_secs)?;
    let at = ts2_at(&out, on)?;
    put_varint(&mut out[at..at + TS2_LEN], now.filetime)?;
    Some(out)
}

#[cfg(windows)]
const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\CloudStore\Store\DefaultAccount\Current\default$windows.data.bluelightreduction.bluelightreductionstate\windows.data.bluelightreduction.bluelightreductionstate";

pub fn refresh() {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    {
        use crate::platform::windows::registry_watch::read_user_binary;
        let state = read_user_binary(KEY, "Data").and_then(|b| parse(&b));
        super::publish_gated(
            service(),
            NightLight { supported: state.is_some(), on: state == Some(true) },
        );
    }
}

/// Switches night light. The registry watch publishes the result.
pub fn set(on: bool) {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    {
        use crate::platform::windows::registry_watch::{read_user_binary, write_user_binary};
        let Some(blob) = read_user_binary(KEY, "Data") else { return };
        let Some(next) = with_state(&blob, on, Now::system()) else {
            eprintln!("barista: night light has a layout this Windows build does not match");
            return;
        };
        if !write_user_binary(KEY, "Data", &next) {
            eprintln!("barista: night light could not be written");
        }
        refresh();
    }
    #[cfg(not(windows))]
    let _ = on;
}

pub fn start() {
    refresh();
    #[cfg(windows)]
    if !crate::platform::windows::registry_watch::watch_user(KEY, refresh) {
        eprintln!("barista: night light not watched");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real values from Windows 11 build 26300 (2026-11-12), written by Windows itself: night light
    // switched off at 20:42:42 and on at 20:44:26.
    const OFF: [u8; 41] = [
        0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0xa2, 0x91, 0xd8, 0xd7, 0x06,
        0x2a, 0x2b, 0x0e, 0x13, 0x43, 0x42, 0x01, 0x00, 0xd0, 0x0a, 0x02, 0xc6, 0x14, 0xbb, 0xab,
        0xa4, 0xcf, 0xd9, 0xda, 0xdc, 0xee, 0x01, 0x00, 0x00, 0x00, 0x00,
    ];
    const ON: [u8; 43] = [
        0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0x8a, 0x92, 0xd8, 0xd7, 0x06,
        0x2a, 0x2b, 0x0e, 0x15, 0x43, 0x42, 0x01, 0x00, 0x10, 0x00, 0xd0, 0x0a, 0x02, 0xc6, 0x14,
        0xa8, 0xe4, 0x9e, 0xbc, 0xdd, 0xda, 0xdc, 0xee, 0x01, 0x00, 0x00, 0x00, 0x00,
    ];
    const T_OFF: Now = Now { unix_secs: 1_794_508_962, filetime: 134_389_825_626_772_923 };
    const T_ON: Now = Now { unix_secs: 1_794_509_066, filetime: 134_389_826_660_577_832 };

    #[test]
    fn reads_both_layouts() {
        assert_eq!(parse(&ON), Some(true));
        assert_eq!(parse(&OFF), Some(false));
    }

    #[test]
    fn switching_on_gives_exactly_what_windows_wrote() {
        assert_eq!(with_state(&OFF, true, T_ON).unwrap(), ON);
    }

    #[test]
    fn switching_off_gives_exactly_what_windows_wrote() {
        assert_eq!(with_state(&ON, false, T_OFF).unwrap(), OFF);
    }

    #[test]
    fn timestamps_are_refreshed_even_when_the_state_is_the_same() {
        let again = with_state(&ON, true, T_OFF).unwrap();
        assert_eq!(again.len(), ON.len());
        assert_eq!(parse(&again), Some(true));
        assert_ne!(again, ON);
    }

    #[test]
    fn unknown_layouts_are_never_touched() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&[0u8; 41]), None);
        let mut other = OFF;
        other[STATE_AT] = 0x12;
        assert_eq!(parse(&other), None);
        assert_eq!(with_state(&other, true, T_ON), None);
        let mut broken_tail = ON;
        broken_tail[PART_AT + 2] = 0x11;
        assert_eq!(parse(&broken_tail), None);
        assert_eq!(parse(&ON[..20]), None);
    }

    #[test]
    fn varints_round_trip_at_fixed_length() {
        let mut b = [0u8; 5];
        put_varint(&mut b, 1_794_509_066).unwrap();
        assert_eq!(b, [0x8a, 0x92, 0xd8, 0xd7, 0x06]);
        assert!(put_varint(&mut [0u8; 2], 1 << 20).is_none());
    }
}
