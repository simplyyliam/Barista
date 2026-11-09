//! Checks on everything a client sends (CLAUDE.md rule 3: never trust a client).
//! Pure functions, so every rule has a quick unit test. Errors are plain messages that go
//! back to the client inside `error { message }`.

use barista_protocol::{
    ClientMessage, ErrorCode, CLIENT_MESSAGE_TYPES, MAX_APP_NAME_LEN, MAX_ICON_BYTES,
    MAX_ICON_PIXELS, MAX_ID_LEN,
};
use base64::{engine::general_purpose::STANDARD, Engine};

/// Why a frame could not become a [`ClientMessage`].
#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub code: ErrorCode,
    pub message: String,
    /// The request's `requestId` if we could read one, so the error can echo it.
    pub request_id: Option<String>,
}

/// The SDK builds request ids as `<item id>#<counter>`, so allow a full-length item id plus that.
const MAX_REQUEST_ID_LEN: usize = MAX_ID_LEN + 16;

/// JSON bytes -> message. Splits "we don't know this type" from "known type, bad fields".
pub fn parse_message(bytes: &[u8]) -> Result<ClientMessage, ParseError> {
    let fail = |code, message: &str, request_id| ParseError {
        code,
        message: message.to_string(),
        request_id,
    };
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| fail(ErrorCode::InvalidMessage, "not valid JSON", None))?;
    // Echo the requestId only if it is a short string, so an error can't be used to bounce
    // a big payload back.
    let request_id = value
        .get("requestId")
        .and_then(|v| v.as_str())
        .filter(|s| s.len() <= MAX_REQUEST_ID_LEN)
        .map(str::to_string);
    let Some(kind) = value.get("type").and_then(|v| v.as_str()) else {
        return Err(fail(
            ErrorCode::InvalidMessage,
            "missing \"type\"",
            request_id,
        ));
    };
    if !CLIENT_MESSAGE_TYPES.contains(&kind) {
        return Err(fail(
            ErrorCode::UnknownType,
            "unknown message type",
            request_id,
        ));
    }
    serde_json::from_value(value).map_err(|e| {
        // serde's message names the field that is wrong, which helps the client developer.
        // It can quote the offending value, so cut it short (like the requestId above).
        ParseError {
            code: ErrorCode::InvalidMessage,
            message: e.to_string().chars().take(200).collect(),
            request_id,
        }
    })
}

/// Item ids: 1..=64 characters from `[a-z0-9-_.]`.
pub fn item_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > MAX_ID_LEN {
        return Err(format!("item id must be 1 to {MAX_ID_LEN} characters"));
    }
    if !id
        .bytes()
        .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.'))
    {
        return Err("item id may only use a-z, 0-9, '-', '_' and '.'".into());
    }
    Ok(())
}

/// Label, tooltip and app name: limited length (in characters), no control characters
/// (they could mess up what the bar draws).
pub fn text(field: &str, value: &str, max_chars: usize) -> Result<(), String> {
    if value.chars().count() > max_chars {
        return Err(format!("{field} is longer than {max_chars} characters"));
    }
    if value.chars().any(char::is_control) {
        return Err(format!("{field} contains control characters"));
    }
    Ok(())
}

pub fn app_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("appName is empty".into());
    }
    text("appName", name, MAX_APP_NAME_LEN)
}

const PNG_PREFIX: &str = "data:image/png;base64,";
const SVG_PREFIX: &str = "data:image/svg+xml;base64,";
const PNG_MAGIC: &[u8] = b"\x89PNG\r\n\x1a\n";

/// An icon is a PNG or SVG `data:` URL, at most 256 KB once decoded. An empty string is
/// allowed (it means "no icon" / "clear the icon").
pub fn icon(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Ok(());
    }
    let (payload, is_png) = if let Some(rest) = value.strip_prefix(PNG_PREFIX) {
        (rest, true)
    } else if let Some(rest) = value.strip_prefix(SVG_PREFIX) {
        (rest, false)
    } else {
        return Err("icon must be a base64 data: URL of a PNG or SVG".into());
    };
    // Check the size before decoding, so we never allocate for a huge payload.
    if payload.len() > MAX_ICON_BYTES / 3 * 4 + 4 {
        return Err(format!("icon is bigger than {} KB", MAX_ICON_BYTES / 1024));
    }
    let bytes = STANDARD
        .decode(payload)
        .map_err(|_| "icon is not valid base64".to_string())?;
    if bytes.len() > MAX_ICON_BYTES {
        return Err(format!("icon is bigger than {} KB", MAX_ICON_BYTES / 1024));
    }
    if is_png {
        if !bytes.starts_with(PNG_MAGIC) {
            return Err("icon says PNG but the data is not a PNG".into());
        }
        // The first chunk of every PNG is IHDR: 4 bytes length, "IHDR", then width and height
        // (big-endian). A small file can claim a huge picture, which the webview would try to
        // decode into gigabytes of memory, so cap the declared size.
        let dimension = |at: usize| {
            bytes
                .get(at..at + 4)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
        };
        match (bytes.get(12..16), dimension(16), dimension(20)) {
            (Some(b"IHDR"), Some(w), Some(h))
                if (1..=MAX_ICON_PIXELS).contains(&w) && (1..=MAX_ICON_PIXELS).contains(&h) => {}
            _ => {
                return Err(format!(
                    "PNG icon must be at most {MAX_ICON_PIXELS}x{MAX_ICON_PIXELS} pixels"
                ))
            }
        }
    } else {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
        let head = head.trim_start_matches('\u{feff}').trim_start();
        if !(head.starts_with("<svg") || head.starts_with("<?xml") || head.starts_with("<!--")) {
            return Err("icon says SVG but the data is not an SVG".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PNG signature plus an IHDR of `w` x `h`, then `extra` filler bytes.
    fn png_sized(w: u32, h: u32, extra: usize) -> String {
        let mut data = PNG_MAGIC.to_vec();
        data.extend_from_slice(&13u32.to_be_bytes());
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&w.to_be_bytes());
        data.extend_from_slice(&h.to_be_bytes());
        data.extend(std::iter::repeat(0u8).take(extra));
        format!("{PNG_PREFIX}{}", STANDARD.encode(data))
    }

    fn png_url(extra: usize) -> String {
        png_sized(16, 16, extra)
    }

    #[test]
    fn item_ids() {
        for ok in ["a", "counter", "a-b_c.d9", &"x".repeat(64)] {
            assert!(item_id(ok).is_ok(), "{ok}");
        }
        for bad in ["", "A", "has space", "a/b", "ä", "a\n", &"x".repeat(65)] {
            assert!(item_id(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn text_limits_and_control_chars() {
        assert!(text("label", "héllo 🌍", 8).is_ok());
        assert!(text("label", "123456789", 8).is_err());
        assert!(text("label", "a\u{0}b", 8).is_err());
        assert!(text("label", "a\nb", 8).is_err());
        assert!(app_name("  ").is_err());
        assert!(app_name("My App").is_ok());
    }

    #[test]
    fn icons() {
        assert!(icon("").is_ok());
        assert!(icon(&png_url(10)).is_ok());
        let svg = format!(
            "{SVG_PREFIX}{}",
            STANDARD.encode("<svg xmlns='http://www.w3.org/2000/svg'/>")
        );
        assert!(icon(&svg).is_ok());
        let xml_svg = format!(
            "{SVG_PREFIX}{}",
            STANDARD.encode("<?xml version='1.0'?><svg/>")
        );
        assert!(icon(&xml_svg).is_ok());
    }

    #[test]
    fn bad_icons_are_rejected() {
        assert!(icon(&png_url(MAX_ICON_BYTES)).is_err(), "too big");
        assert!(icon("https://example.com/a.png").is_err(), "not a data url");
        assert!(icon("data:image/gif;base64,R0lGOD").is_err(), "wrong type");
        assert!(icon(&format!("{PNG_PREFIX}%%%")).is_err(), "bad base64");
        assert!(
            icon(&format!("{PNG_PREFIX}{}", STANDARD.encode("not a png"))).is_err(),
            "bad magic"
        );
        assert!(
            icon(&format!("{SVG_PREFIX}{}", STANDARD.encode("<html></html>"))).is_err(),
            "not svg"
        );
        assert!(
            icon(&format!("{PNG_PREFIX}{}", "A".repeat(900_000))).is_err(),
            "huge payload"
        );
    }

    #[test]
    fn png_dimensions_are_capped() {
        assert!(icon(&png_sized(1024, 1024, 0)).is_ok());
        assert!(icon(&png_sized(1025, 16, 0)).is_err(), "too wide");
        assert!(icon(&png_sized(16, 30000, 0)).is_err(), "too tall");
        assert!(icon(&png_sized(0, 16, 0)).is_err(), "zero width");
        assert!(
            icon(&png_sized(30000, 30000, 0)).is_err(),
            "a small file claiming a huge picture"
        );
        let magic_only = format!("{PNG_PREFIX}{}", STANDARD.encode(PNG_MAGIC));
        assert!(icon(&magic_only).is_err(), "no IHDR at all");
    }

    #[test]
    fn serde_errors_are_cut_short() {
        let big = "x".repeat(100_000);
        let text = format!(r#"{{"type":"item.create","id":"a","label":{big:?},"icon":5}}"#);
        let error = parse_message(text.as_bytes()).unwrap_err();
        assert!(error.message.chars().count() <= 200);
        let text = format!(r#"{{"type":"item.remove","id":{big:?}}}"#);
        assert!(
            parse_message(text.as_bytes()).is_ok(),
            "long ids parse; the id rule rejects them later"
        );
    }

    #[test]
    fn parse_tells_unknown_from_invalid() {
        let e = parse_message(b"nope").unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidMessage);
        let e = parse_message(br#"{"id":"a"}"#).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidMessage);
        let e = parse_message(br#"{"type":"item.teleport","requestId":"r1"}"#).unwrap_err();
        assert_eq!(
            (e.code, e.request_id.as_deref()),
            (ErrorCode::UnknownType, Some("r1"))
        );
        let e = parse_message(br#"{"type":"item.create","requestId":"r2"}"#).unwrap_err();
        assert_eq!(
            (e.code, e.request_id.as_deref()),
            (ErrorCode::InvalidMessage, Some("r2"))
        );
        let long = format!(r#"{{"type":"x","requestId":"{}"}}"#, "r".repeat(100));
        let ok = format!(r#"{{"type":"x","requestId":"{}#12"}}"#, "r".repeat(64));
        assert!(parse_message(ok.as_bytes())
            .unwrap_err()
            .request_id
            .is_some());
        assert_eq!(parse_message(long.as_bytes()).unwrap_err().request_id, None);
        assert!(parse_message(br#"{"type":"item.remove","id":"a"}"#).is_ok());
    }
}
