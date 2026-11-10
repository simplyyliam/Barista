//! Checks on everything a client sends (CLAUDE.md rule 3: never trust a client).
//! Pure functions, so every rule has a quick unit test. Errors are plain messages that go
//! back to the client inside `error { message }`.

use barista_protocol::{
    ClientMessage, ErrorCode, MenuTemplateItem, MenuItemType, CLIENT_MESSAGE_TYPES, MAX_ACCELERATOR_LEN,
    MAX_APP_NAME_LEN, MAX_ICON_BYTES, MAX_ICON_PIXELS, MAX_ID_LEN, MAX_LABEL_LEN, MAX_MENU_DEPTH,
    MAX_MENU_ICON_BYTES, MAX_MENU_ITEMS,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::collections::HashSet;

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
    icon_up_to(value, MAX_ICON_BYTES)
}

fn icon_up_to(value: &str, max_bytes: usize) -> Result<(), String> {
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
    if payload.len() > max_bytes / 3 * 4 + 4 {
        return Err(format!("icon is bigger than {} KB", max_bytes / 1024));
    }
    let bytes = STANDARD
        .decode(payload)
        .map_err(|_| "icon is not valid base64".to_string())?;
    if bytes.len() > max_bytes {
        return Err(format!("icon is bigger than {} KB", max_bytes / 1024));
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

/// A menu template: depth at most 3, at most 50 entries in the whole tree, ids on every entry
/// but separators (same charset as item ids) and unique in the whole tree, plus the usual text
/// and icon rules. Errors name the entry, so the developer can find it.
pub fn menu(items: &[MenuTemplateItem]) -> Result<(), String> {
    let mut seen = HashSet::new();
    let mut count = 0;
    menu_level(items, 1, &mut seen, &mut count)
}

// `'a` says: the ids in `seen` borrow from `items`, so no id string is copied.
fn menu_level<'a>(
    items: &'a [MenuTemplateItem],
    depth: usize,
    seen: &mut HashSet<&'a str>,
    count: &mut usize,
) -> Result<(), String> {
    if depth > MAX_MENU_DEPTH {
        return Err(format!(
            "menus can be nested at most {MAX_MENU_DEPTH} levels deep"
        ));
    }
    for item in items {
        *count += 1;
        if *count > MAX_MENU_ITEMS {
            return Err(format!("a menu can have at most {MAX_MENU_ITEMS} entries"));
        }
        let kind = item.kind();
        let name = item.id.as_deref().unwrap_or("(separator)");
        match &item.id {
            None if kind == MenuItemType::Separator => {}
            None => return Err("every menu entry except separators needs an id".into()),
            Some(id) => {
                item_id(id).map_err(|m| format!("menu entry {id:?}: {m}"))?;
                if !seen.insert(id) {
                    return Err(format!("menu id {id:?} is used twice"));
                }
            }
        }
        let field = |f: &str| format!("menu entry {name:?}: {f}");
        if let Some(label) = &item.label {
            text("label", label, MAX_LABEL_LEN).map_err(|m| field(&m))?;
        }
        if let Some(accelerator) = &item.accelerator {
            text("accelerator", accelerator, MAX_ACCELERATOR_LEN).map_err(|m| field(&m))?;
        }
        if let Some(icon) = &item.icon {
            icon_up_to(icon, MAX_MENU_ICON_BYTES).map_err(|m| field(&m))?;
        }
        if item.checked.is_some() && !matches!(kind, MenuItemType::Checkbox | MenuItemType::Radio) {
            return Err(field("only checkbox and radio entries can be checked"));
        }
        match (kind, &item.submenu) {
            (MenuItemType::Submenu, Some(sub)) => menu_level(sub, depth + 1, seen, count)?,
            (MenuItemType::Submenu, None) => {
                return Err(field("a submenu entry needs a submenu list"))
            }
            (_, Some(_)) => return Err(field("only submenu entries can have a submenu")),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> MenuTemplateItem {
        MenuTemplateItem {
            id: Some(id.into()),
            label: Some(id.into()),
            ..Default::default()
        }
    }

    fn sub(id: &str, items: Vec<MenuTemplateItem>) -> MenuTemplateItem {
        MenuTemplateItem {
            submenu: Some(items),
            ..entry(id)
        }
    }

    fn separator() -> MenuTemplateItem {
        MenuTemplateItem {
            kind: Some(MenuItemType::Separator),
            ..Default::default()
        }
    }

    #[test]
    fn good_menus_pass() {
        assert!(menu(&[]).is_ok());
        let checkbox = MenuTemplateItem {
            kind: Some(MenuItemType::Checkbox),
            checked: Some(true),
            accelerator: Some("Ctrl+S".into()),
            icon: Some(png_url(0)),
            ..entry("sound")
        };
        let three_deep = sub("b", vec![sub("c", vec![entry("d")])]);
        let ok = [entry("a"), separator(), checkbox, separator(), three_deep];
        assert_eq!(menu(&ok), Ok(()));
    }

    #[test]
    fn menu_depth_is_capped_at_three() {
        let four = [sub("a", vec![sub("b", vec![sub("c", vec![entry("d")])])])];
        assert!(menu(&four).unwrap_err().contains("levels deep"));
    }

    #[test]
    fn menu_size_is_capped_including_submenus() {
        let fifty: Vec<_> = (0..50).map(|i| entry(&format!("e{i}"))).collect();
        assert!(menu(&fifty).is_ok());
        let mut nested: Vec<_> = (0..49).map(|i| entry(&format!("e{i}"))).collect();
        nested.push(sub("s", vec![entry("x")]));
        assert!(menu(&nested).unwrap_err().contains("at most 50"));
    }

    #[test]
    fn menu_ids_are_required_valid_and_unique() {
        let no_id = MenuTemplateItem {
            label: Some("x".into()),
            ..Default::default()
        };
        assert!(menu(&[no_id]).unwrap_err().contains("needs an id"));
        assert!(menu(&[entry("Bad Id")]).is_err());
        let twice = [entry("a"), sub("s", vec![entry("a")])];
        assert!(menu(&twice).unwrap_err().contains("twice"));
        assert!(menu(&[separator(), separator()]).is_ok());
    }

    #[test]
    fn menu_fields_are_checked() {
        let long = MenuTemplateItem {
            accelerator: Some("x".repeat(33)),
            ..entry("a")
        };
        assert!(menu(&[long]).is_err());
        let control = MenuTemplateItem {
            label: Some("a\u{7}".into()),
            ..entry("a")
        };
        assert!(menu(&[control]).is_err());
        let big_icon = MenuTemplateItem {
            icon: Some(png_url(20 * 1024)),
            ..entry("a")
        };
        assert!(menu(&[big_icon]).unwrap_err().contains("16 KB"));
        let checked_normal = MenuTemplateItem {
            checked: Some(true),
            ..entry("a")
        };
        assert!(menu(&[checked_normal]).is_err());
        let empty_sub = MenuTemplateItem {
            kind: Some(MenuItemType::Submenu),
            ..entry("a")
        };
        assert!(menu(&[empty_sub]).is_err());
        let normal_with_sub = MenuTemplateItem {
            kind: Some(MenuItemType::Normal),
            ..sub("a", vec![])
        };
        assert!(menu(&[normal_with_sub]).is_err());
    }

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
