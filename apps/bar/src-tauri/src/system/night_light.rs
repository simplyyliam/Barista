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

/// Bytes 0 to 9 are the same in every sample: a header and the start of the timestamp field.
const HEADER: [u8; 10] = [0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06];
/// Where the state byte sits, and the bytes right before it.
const STATE_AT: usize = 18;
const MARKER: [u8; 3] = [0x2a, 0x2b, 0x0e];
const MARKER_AT: usize = 15;
/// After `43 42 01 00` (bytes 19 to 22) comes the part that differs between on and off.
const PART_AT: usize = 23;
const OFF_STATE: u8 = 0x13;
const OFF_PART: [u8; 3] = [0xd0, 0x0a, 0x02];
const ON_STATE: u8 = 0x12;
const ON_PART: [u8; 2] = [0x10, 0x00];

fn layout_ok(blob: &[u8]) -> bool {
    blob.len() > PART_AT + 3
        && blob[..HEADER.len()] == HEADER
        && blob[MARKER_AT..MARKER_AT + 3] == MARKER
}

/// `Some(true)` = on, `Some(false)` = off, `None` = a layout we don't know.
pub fn parse(blob: &[u8]) -> Option<bool> {
    if !layout_ok(blob) {
        return None;
    }
    match blob[STATE_AT] {
        OFF_STATE if blob[PART_AT..PART_AT + 3] == OFF_PART => Some(false),
        ON_STATE if blob[PART_AT..PART_AT + 2] == ON_PART => Some(true),
        _ => None,
    }
}

/// The same blob with night light switched on or off. `None` if the layout is unknown.
pub fn with_state(blob: &[u8], on: bool) -> Option<Vec<u8>> {
    let current = parse(blob)?;
    if current == on {
        return Some(blob.to_vec());
    }
    let (state, old_len, part): (u8, usize, &[u8]) =
        if on { (ON_STATE, OFF_PART.len(), &ON_PART) } else { (OFF_STATE, ON_PART.len(), &OFF_PART) };
    let mut out = blob.to_vec();
    out[STATE_AT] = state;
    out.splice(PART_AT..PART_AT + old_len, part.iter().copied());
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
    #[cfg(windows)]
    {
        use crate::platform::windows::registry_watch::{read_user_binary, write_user_binary};
        let Some(blob) = read_user_binary(KEY, "Data") else { return };
        let Some(next) = with_state(&blob, on) else {
            eprintln!("barista: night light has a layout this Windows build does not match");
            return;
        };
        if next != blob && !write_user_binary(KEY, "Data", &next) {
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

    // Real values from Windows 11 build 26300 (2026-11-12): one with night light on, one off.
    const ON: [u8; 40] = [
        0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0xa0, 0xfd, 0xd7, 0xd7, 0x06,
        0x2a, 0x2b, 0x0e, 0x12, 0x43, 0x42, 0x01, 0x00, 0x10, 0x00, 0xc6, 0x14, 0xc8, 0xe2, 0xc5,
        0x93, 0xfa, 0xd9, 0xdc, 0xee, 0x01, 0x00, 0x00, 0x00, 0x00,
    ];
    const OFF: [u8; 41] = [
        0x43, 0x42, 0x01, 0x00, 0x0a, 0x02, 0x01, 0x00, 0x2a, 0x06, 0xa0, 0x88, 0xd8, 0xd7, 0x06,
        0x2a, 0x2b, 0x0e, 0x13, 0x43, 0x42, 0x01, 0x00, 0xd0, 0x0a, 0x02, 0xc6, 0x14, 0xd2, 0xe5,
        0xd4, 0xd0, 0xae, 0xda, 0xdc, 0xee, 0x01, 0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn reads_both_known_layouts() {
        assert_eq!(parse(&ON), Some(true));
        assert_eq!(parse(&OFF), Some(false));
    }

    #[test]
    fn switching_changes_only_the_state_part() {
        let on = with_state(&OFF, true).unwrap();
        assert_eq!(parse(&on), Some(true));
        assert_eq!(on.len(), OFF.len() - 1);
        // Everything before and after the changed part is untouched.
        assert_eq!(on[..STATE_AT], OFF[..STATE_AT]);
        assert_eq!(on[PART_AT + 2..], OFF[PART_AT + 3..]);

        let off = with_state(&ON, false).unwrap();
        assert_eq!(parse(&off), Some(false));
        assert_eq!(off.len(), ON.len() + 1);
        assert_eq!(off[PART_AT + 3..], ON[PART_AT + 2..]);
    }

    #[test]
    fn a_round_trip_gives_the_same_bytes() {
        assert_eq!(with_state(&with_state(&OFF, true).unwrap(), false).unwrap(), OFF);
        assert_eq!(with_state(&with_state(&ON, false).unwrap(), true).unwrap(), ON);
    }

    #[test]
    fn same_state_changes_nothing() {
        assert_eq!(with_state(&ON, true).unwrap(), ON);
    }

    #[test]
    fn unknown_layouts_are_never_touched() {
        assert_eq!(parse(&[]), None);
        assert_eq!(parse(&[0u8; 41]), None);
        let mut other = OFF;
        other[STATE_AT] = 0x15;
        assert_eq!(parse(&other), None);
        assert_eq!(with_state(&other, true), None);
        let mut short = ON.to_vec();
        short.truncate(20);
        assert_eq!(parse(&short), None);
    }
}
