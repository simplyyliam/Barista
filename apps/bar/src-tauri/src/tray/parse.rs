//! Reads the message `Shell_NotifyIcon` sends to the tray window (ADR 0012, 0026). Pure Rust, no
//! Windows calls, so it can be tested with plain byte arrays.
//!
//! The payload is not documented but has been frozen since Windows 95. It is the `WM_COPYDATA`
//! `lpData`: a signature, the `NIM_*` verb, then a `NOTIFYICONDATAW` where the two handles (window
//! and icon) are 32-bit even on 64-bit Windows, so the layout is the same everywhere:
//!
//! ```text
//!  offset in the icon data   field
//!    0   cbSize             4   (the app's size, we don't trust it for the layout)
//!    4   hWnd               4   32-bit handle
//!    8   uID                4
//!   12   uFlags             4
//!   16   uCallbackMessage   4
//!   20   hIcon              4   32-bit handle
//!   24   szTip              128 UTF-16 units (older apps only fill the first 64)
//!  280   dwState            4
//!  284   dwStateMask        4
//!  288   szInfo             256 units (balloon text, ignored)
//!  800   uVersion / uTimeout 4
//!  804   szInfoTitle        64 units (ignored)
//!  932   dwInfoFlags        4
//!  936   guidItem           16
//!  952   hBalloonIcon       4
//! ```
//!
//! Rule 3: this comes from any process on the machine, so every read checks the length first and
//! nothing here can panic on odd input.

/// First four bytes of every tray payload.
pub const SIGNATURE: u32 = 0x3475_3423;
/// `COPYDATASTRUCT.dwData` of a tray message (0 is the AppBar API, 3 asks for an icon's rect).
pub const COPYDATA_TRAY: usize = 1;

const HEADER: usize = 8;
const OFF_HWND: usize = 4;
const OFF_ID: usize = 8;
const OFF_FLAGS: usize = 12;
const OFF_CALLBACK: usize = 16;
const OFF_ICON: usize = 20;
const OFF_TIP: usize = 24;
const TIP_V1_UNITS: usize = 64;
const TIP_UNITS: usize = 128;
const OFF_STATE: usize = 280;
const OFF_STATE_MASK: usize = 284;
const OFF_VERSION: usize = 800;
const OFF_GUID: usize = 936;
/// Biggest payload we look at (the full struct plus the header). Anything bigger is cut off, not
/// rejected: newer Windows versions only ever add fields at the end.
const MAX_LEN: usize = HEADER + 1024;

const NIF_MESSAGE: u32 = 0x01;
const NIF_ICON: u32 = 0x02;
const NIF_TIP: u32 = 0x04;
const NIF_STATE: u32 = 0x08;
const NIF_GUID: u32 = 0x20;
const NIS_HIDDEN: u32 = 0x01;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Add,
    Modify,
    Delete,
    SetFocus,
    SetVersion,
}

/// Which icon a message is about: the app's own window and id, or the GUID it registered with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconKey {
    Handle { hwnd: u32, id: u32 },
    Guid([u8; 16]),
}

impl IconKey {
    /// Stable text for the registry (`tray/<this>`): the GUID, or the window and id in hex.
    pub fn text(&self) -> String {
        match self {
            Self::Handle { hwnd, id } => format!("{hwnd:x}-{id:x}"),
            Self::Guid(g) => g.iter().map(|b| format!("{b:02x}")).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayMessage {
    pub action: Action,
    pub key: IconKey,
    /// The icon owner's window (32-bit handle, widen it with `as isize` sign extension off).
    pub hwnd: u32,
    pub id: u32,
    /// `Some` when the message carries a callback message number (`NIF_MESSAGE`).
    pub callback_message: Option<u32>,
    /// `Some` when the message carries an icon (`NIF_ICON`). 0 means "no icon".
    pub icon_handle: Option<u32>,
    /// `Some` when the message carries a tooltip (`NIF_TIP`); empty means "no tooltip".
    pub tip: Option<String>,
    /// `Some` when the message sets the hidden state (`NIF_STATE`).
    pub hidden: Option<bool>,
    /// For `Action::SetVersion`.
    pub version: Option<u32>,
}

fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    let bytes = data.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

/// UTF-16 text up to the first NUL (or `units` characters), `None` if it isn't valid UTF-16.
fn utf16_at(data: &[u8], offset: usize, units: usize) -> Option<String> {
    let mut chars = Vec::new();
    for i in 0..units {
        let at = offset + i * 2;
        let pair = data.get(at..at + 2)?;
        let unit = u16::from_le_bytes([pair[0], pair[1]]);
        if unit == 0 {
            break;
        }
        chars.push(unit);
    }
    String::from_utf16(&chars).ok()
}

/// Parses a tray payload. `None` for anything that isn't a well-formed message: the caller still
/// forwards it to Explorer, we just don't show it.
pub fn parse(data: &[u8]) -> Option<TrayMessage> {
    let data = &data[..data.len().min(MAX_LEN)];
    if u32_at(data, 0)? != SIGNATURE {
        return None;
    }
    let action = match u32_at(data, 4)? {
        0 => Action::Add,
        1 => Action::Modify,
        2 => Action::Delete,
        3 => Action::SetFocus,
        4 => Action::SetVersion,
        _ => return None,
    };
    let nid = &data[HEADER..];
    let hwnd = u32_at(nid, OFF_HWND)?;
    let id = u32_at(nid, OFF_ID)?;
    let flags = u32_at(nid, OFF_FLAGS)?;

    let key = if flags & NIF_GUID != 0 {
        let bytes = nid.get(OFF_GUID..OFF_GUID + 16)?;
        IconKey::Guid(bytes.try_into().ok()?)
    } else {
        IconKey::Handle { hwnd, id }
    };

    let callback_message = if flags & NIF_MESSAGE != 0 {
        Some(u32_at(nid, OFF_CALLBACK)?)
    } else {
        None
    };
    let icon_handle = if flags & NIF_ICON != 0 {
        Some(u32_at(nid, OFF_ICON)?)
    } else {
        None
    };
    let tip = if flags & NIF_TIP != 0 {
        // A short payload is an old (V1) struct with a 64 character tip.
        let units = if nid.len() >= OFF_TIP + TIP_UNITS * 2 {
            TIP_UNITS
        } else {
            TIP_V1_UNITS
        };
        Some(utf16_at(nid, OFF_TIP, units)?)
    } else {
        None
    };
    let hidden = if flags & NIF_STATE != 0 {
        // Only the bits the app says it is changing count.
        match (u32_at(nid, OFF_STATE), u32_at(nid, OFF_STATE_MASK)) {
            (Some(state), Some(mask)) if mask & NIS_HIDDEN != 0 => Some(state & NIS_HIDDEN != 0),
            _ => None,
        }
    } else {
        None
    };
    let version = if action == Action::SetVersion {
        Some(u32_at(nid, OFF_VERSION)?)
    } else {
        None
    };

    Some(TrayMessage {
        action,
        key,
        hwnd,
        id,
        callback_message,
        icon_handle,
        tip,
        hidden,
        version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A payload with the given verb and flags; `nid_len` is the length of the icon data.
    fn payload(verb: u32, flags: u32, nid_len: usize) -> Vec<u8> {
        let mut data = vec![0u8; HEADER + nid_len];
        data[0..4].copy_from_slice(&SIGNATURE.to_le_bytes());
        data[4..8].copy_from_slice(&verb.to_le_bytes());
        put(&mut data, OFF_HWND, 0x1234);
        put(&mut data, OFF_ID, 7);
        put(&mut data, OFF_FLAGS, flags);
        data
    }

    fn put(data: &mut [u8], nid_offset: usize, value: u32) {
        let at = HEADER + nid_offset;
        data[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_text(data: &mut [u8], nid_offset: usize, text: &str) {
        let mut at = HEADER + nid_offset;
        for unit in text.encode_utf16() {
            data[at..at + 2].copy_from_slice(&unit.to_le_bytes());
            at += 2;
        }
    }

    const FULL: usize = 956;
    const V1: usize = 152;

    #[test]
    fn add_with_everything() {
        let mut d = payload(0, NIF_MESSAGE | NIF_ICON | NIF_TIP, FULL);
        put(&mut d, OFF_CALLBACK, 0x8001);
        put(&mut d, OFF_ICON, 0x55);
        put_text(&mut d, OFF_TIP, "Discord");
        let m = parse(&d).unwrap();
        assert_eq!(m.action, Action::Add);
        assert_eq!(m.key, IconKey::Handle { hwnd: 0x1234, id: 7 });
        assert_eq!(m.callback_message, Some(0x8001));
        assert_eq!(m.icon_handle, Some(0x55));
        assert_eq!(m.tip.as_deref(), Some("Discord"));
        assert_eq!(m.hidden, None);
    }

    #[test]
    fn only_the_flagged_fields_are_read() {
        let mut d = payload(1, NIF_TIP, FULL);
        put(&mut d, OFF_ICON, 0x99);
        put_text(&mut d, OFF_TIP, "new tip");
        let m = parse(&d).unwrap();
        assert_eq!(m.action, Action::Modify);
        assert_eq!(m.icon_handle, None);
        assert_eq!(m.callback_message, None);
        assert_eq!(m.tip.as_deref(), Some("new tip"));
    }

    #[test]
    fn old_v1_struct_has_a_64_char_tip() {
        let mut d = payload(0, NIF_TIP, V1);
        put_text(&mut d, OFF_TIP, "old app");
        assert_eq!(parse(&d).unwrap().tip.as_deref(), Some("old app"));
    }

    #[test]
    fn tip_without_a_nul_stops_at_the_field_end() {
        let mut d = payload(0, NIF_TIP, FULL);
        put_text(&mut d, OFF_TIP, &"a".repeat(TIP_UNITS));
        // The bytes right after the tip (dwState) must not leak in.
        put(&mut d, OFF_STATE, 0x6161_6161);
        let tip = parse(&d).unwrap().tip.unwrap();
        assert_eq!(tip.len(), TIP_UNITS);
    }

    #[test]
    fn broken_utf16_is_rejected() {
        let mut d = payload(0, NIF_TIP, FULL);
        let at = HEADER + OFF_TIP;
        d[at..at + 2].copy_from_slice(&0xD800u16.to_le_bytes()); // lone surrogate
        assert_eq!(parse(&d), None);
    }

    #[test]
    fn wrong_signature_and_unknown_verb() {
        let mut d = payload(0, 0, FULL);
        d[0] = 0;
        assert_eq!(parse(&d), None);
        assert_eq!(parse(&payload(9, 0, FULL)), None);
    }

    #[test]
    fn truncated_payloads_do_not_panic() {
        let full = payload(0, NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_STATE | NIF_GUID, FULL);
        for len in 0..full.len() {
            let _ = parse(&full[..len]);
        }
        // Flags that promise more than the payload holds are refused.
        assert_eq!(parse(&payload(0, NIF_GUID, 40)), None);
        assert_eq!(parse(&payload(0, NIF_ICON, 18)), None);
    }

    #[test]
    fn oversized_payload_is_cut_not_rejected() {
        let mut d = payload(0, NIF_ICON, FULL);
        d.extend(std::iter::repeat(0xAB).take(100_000));
        put(&mut d, OFF_ICON, 5);
        assert_eq!(parse(&d).unwrap().icon_handle, Some(5));
    }

    #[test]
    fn guid_is_the_key_when_flagged() {
        let mut d = payload(0, NIF_GUID, FULL);
        for i in 0..16 {
            d[HEADER + OFF_GUID + i] = i as u8 + 1;
        }
        let m = parse(&d).unwrap();
        assert!(matches!(m.key, IconKey::Guid(g) if g[0] == 1 && g[15] == 16));
        assert_eq!(m.key.text().len(), 32);
    }

    #[test]
    fn hidden_flag_set_and_cleared() {
        let mut d = payload(1, NIF_STATE, FULL);
        put(&mut d, OFF_STATE, NIS_HIDDEN);
        put(&mut d, OFF_STATE_MASK, NIS_HIDDEN);
        assert_eq!(parse(&d).unwrap().hidden, Some(true));
        put(&mut d, OFF_STATE, 0);
        assert_eq!(parse(&d).unwrap().hidden, Some(false));
        // Mask without the hidden bit: the app isn't talking about it.
        put(&mut d, OFF_STATE_MASK, 2);
        assert_eq!(parse(&d).unwrap().hidden, None);
    }

    #[test]
    fn set_version_reads_the_version() {
        let mut d = payload(4, 0, FULL);
        put(&mut d, OFF_VERSION, 4);
        let m = parse(&d).unwrap();
        assert_eq!(m.action, Action::SetVersion);
        assert_eq!(m.version, Some(4));
    }

    #[test]
    fn handle_key_text_is_stable() {
        let key = IconKey::Handle { hwnd: 0xAB, id: 0x10 };
        assert_eq!(key.text(), "ab-10");
    }
}
