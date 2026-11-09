//! The Barista protocol: every message between the bar and a client app.
//!
//! These types are the single source of truth. `pnpm gen` (the `export_protocol_types` test
//! below) turns them into `packages/sdk/src/protocol.ts`, so the Rust and TypeScript sides
//! can never drift apart. See docs/release/plan/03-protocol.md for the full message list.
//!
//! Wire format: JSON with a `type` field, e.g. `{"type":"hello","protocolVersion":"0.1",...}`.

use serde::{Deserialize, Serialize};
use specta::Type;

/// `major.minor`. The bar accepts any client with the same major version.
/// New optional fields bump the minor; removing or renaming a field bumps the major.
pub const PROTOCOL_VERSION: &str = "0.2";

/// Largest frame (the JSON part) either side accepts.
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
/// Largest icon after base64 decoding (PNG or SVG).
pub const MAX_ICON_BYTES: usize = 256 * 1024;
pub const MAX_ID_LEN: usize = 64;
pub const MAX_LABEL_LEN: usize = 128;
pub const MAX_TOOLTIP_LEN: usize = 512;
pub const MAX_APP_NAME_LEN: usize = 64;
pub const MAX_ITEMS_PER_CLIENT: usize = 8;

/// Every `type` a client may send. The bar uses it to tell `unknown_type` (a newer client)
/// from `invalid_message` (a known type with wrong fields). A test keeps it in sync.
pub const CLIENT_MESSAGE_TYPES: [&str; 6] = [
    "hello",
    "item.create",
    "item.update",
    "item.remove",
    "debug.listItems",
    "debug.click",
];

/// Which SDK a client was built with (shown in the bar's debug info).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct SdkInfo {
    pub name: String,
    pub version: String,
}

/// Which mouse button clicked an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// One item as the bar knows it. Used in `debug.items` replies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ItemInfo {
    pub id: String,
    pub app_name: String,
    pub pid: u32,
    /// Only whether there is one: icons can be big, and a reply must stay under the frame limit.
    pub has_icon: bool,
    pub label: Option<String>,
    pub tooltip: Option<String>,
    pub visible: bool,
}

/// Client -> bar. Every request may carry a `requestId`; errors echo it.
///
/// `icon` is a `data:` URL (`image/png` or `image/svg+xml`, base64).
/// In `item.update`, a missing field means "unchanged" and an empty string clears it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum ClientMessage {
    /// First message on a new connection.
    #[serde(rename = "hello")]
    Hello {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        protocol_version: String,
        app_name: String,
        pid: u32,
        sdk: SdkInfo,
    },
    #[serde(rename = "item.create")]
    ItemCreate {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tooltip: Option<String>,
    },
    #[serde(rename = "item.update")]
    ItemUpdate {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        icon: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tooltip: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        visible: Option<bool>,
    },
    #[serde(rename = "item.remove")]
    ItemRemove {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        id: String,
    },
    /// Asks for every item in the bar (used by `examples/inspect` and the bench). Debug builds,
    /// or a bar started with `BARISTA_DEBUG` set; otherwise `unknown_type`.
    #[serde(rename = "debug.listItems")]
    DebugListItems {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
    },
    /// Fakes a click on any item. `id` is the item's key from `debug.items`, like `s3/counter`.
    /// Same availability as `debug.listItems`.
    #[serde(rename = "debug.click")]
    DebugClick {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        id: String,
        button: MouseButton,
    },
}

impl ClientMessage {
    /// The `requestId` of any request, so an error can echo it.
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::Hello { request_id, .. }
            | Self::ItemCreate { request_id, .. }
            | Self::ItemUpdate { request_id, .. }
            | Self::ItemRemove { request_id, .. }
            | Self::DebugListItems { request_id }
            | Self::DebugClick { request_id, .. } => request_id.as_deref(),
        }
    }
}

/// Bar -> client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum BarMessage {
    /// Reply to an accepted `hello`.
    #[serde(rename = "welcome")]
    Welcome {
        session_id: String,
        bar_version: String,
        protocol_version: String,
    },
    /// Something went wrong. `request_id` echoes the request it answers, if there was one.
    #[serde(rename = "error")]
    Error {
        request_id: Option<String>,
        code: ErrorCode,
        message: String,
    },
    /// The user clicked one of the client's items. `id` is the client's own item id.
    #[serde(rename = "item.click")]
    ItemClick { id: String, button: MouseButton },
    /// Reply to `debug.listItems`.
    #[serde(rename = "debug.items")]
    DebugItems {
        request_id: Option<String>,
        items: Vec<ItemInfo>,
    },
}

/// Machine-readable reason in an `error` message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The message `type` is not one the bar knows (a newer client talking to an older bar).
    UnknownType,
    /// The message is not valid JSON or breaks a size/id rule.
    InvalidMessage,
    /// The client's protocol major version differs from the bar's.
    VersionMismatch,
    /// A request arrived before `hello` was accepted.
    NotReady,
    /// `item.create` with an id this client already uses.
    DuplicateId,
    /// `item.update` / `item.remove` for an id that does not exist.
    NotFound,
    /// The client already has the maximum number of items.
    LimitReached,
    /// The icon is not a valid PNG/SVG data URL or is too big.
    InvalidIcon,
    /// Too many invalid messages in a short time; the bar closes the connection.
    TooManyErrors,
}

/// True when `client` has the same major version as this crate's [`PROTOCOL_VERSION`].
pub fn is_compatible(client: &str) -> bool {
    // A version is exactly `major.minor`, both numbers. Anything else is rejected.
    let parse = |v: &str| -> Option<(u32, u32)> {
        let (major, minor) = v.split_once('.')?;
        Some((major.parse().ok()?, minor.parse().ok()?))
    };
    match (parse(client), parse(PROTOCOL_VERSION)) {
        (Some((a, _)), Some((b, _))) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use specta::Types;
    use specta_typescript::Typescript;

    /// Where the generated file goes, relative to this crate (crates/protocol).
    const OUTPUT: &str = "../../packages/sdk/src/protocol.ts";

    /// `pnpm gen` runs this. CI then fails on `git diff` if the committed file was stale.
    #[test]
    fn export_protocol_types() {
        let types = Types::default()
            .register::<ClientMessage>()
            .register::<BarMessage>();
        Typescript::default()
            .header(format!(
                "// GENERATED by crates/protocol (pnpm gen). Do not edit by hand.\n\
                 // @ts-nocheck\n\
                 export const PROTOCOL_VERSION = \"{PROTOCOL_VERSION}\"\n\
                 export const MAX_MESSAGE_BYTES = {MAX_MESSAGE_BYTES}\n\
                 export const MAX_ICON_BYTES = {MAX_ICON_BYTES}\n\
                 export const MAX_ID_LEN = {MAX_ID_LEN}\n\
                 export const MAX_LABEL_LEN = {MAX_LABEL_LEN}\n\
                 export const MAX_TOOLTIP_LEN = {MAX_TOOLTIP_LEN}\n\
                 export const MAX_APP_NAME_LEN = {MAX_APP_NAME_LEN}\n\
                 export const MAX_ITEMS_PER_CLIENT = {MAX_ITEMS_PER_CLIENT}\n"
            ))
            .export_to(OUTPUT, &types, specta_serde::Format)
            .expect("failed to export protocol types");
    }

    #[test]
    fn hello_has_the_documented_json_shape() {
        let hello = ClientMessage::Hello {
            request_id: None,
            protocol_version: PROTOCOL_VERSION.into(),
            app_name: "Test".into(),
            pid: 42,
            sdk: SdkInfo {
                name: "@barista/sdk".into(),
                version: "0.0.0".into(),
            },
        };
        let value = serde_json::to_value(&hello).unwrap();
        assert_eq!(
            value,
            json!({
                "type": "hello",
                "protocolVersion": PROTOCOL_VERSION,
                "appName": "Test",
                "pid": 42,
                "sdk": { "name": "@barista/sdk", "version": "0.0.0" }
            })
        );
        assert_eq!(
            serde_json::from_value::<ClientMessage>(value).unwrap(),
            hello
        );
    }

    #[test]
    fn welcome_and_error_round_trip() {
        let welcome = BarMessage::Welcome {
            session_id: "s1".into(),
            bar_version: "0.0.0".into(),
            protocol_version: PROTOCOL_VERSION.into(),
        };
        let error = BarMessage::Error {
            request_id: Some("r1".into()),
            code: ErrorCode::UnknownType,
            message: "nope".into(),
        };
        for msg in [welcome, error] {
            let text = serde_json::to_string(&msg).unwrap();
            assert_eq!(serde_json::from_str::<BarMessage>(&text).unwrap(), msg);
        }
        let value = serde_json::to_value(BarMessage::Error {
            request_id: None,
            code: ErrorCode::VersionMismatch,
            message: String::new(),
        })
        .unwrap();
        assert_eq!(value["code"], "version_mismatch");
        assert_eq!(value["type"], "error");
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let text = r#"{"type":"hello","protocolVersion":"0.2","appName":"A","pid":1,
            "sdk":{"name":"x","version":"1"},"somethingNew":true}"#;
        assert!(serde_json::from_str::<ClientMessage>(text).is_ok());
    }

    #[test]
    fn unknown_message_type_fails_to_parse() {
        // The IPC layer (v0.2) turns this failure into `error { code: "unknown_type" }`.
        assert!(serde_json::from_str::<ClientMessage>(r#"{"type":"item.teleport"}"#).is_err());
    }

    #[test]
    fn same_major_version_is_compatible() {
        assert!(is_compatible("0.1"));
        assert!(is_compatible("0.2"));
        assert!(is_compatible("0.9"));
        assert!(!is_compatible("1.0"));
        assert!(!is_compatible(""));
        assert!(!is_compatible("0"));
        assert!(!is_compatible("0.x"));
        assert!(!is_compatible("0.1.2"));
    }

    fn round_trip_client(msg: ClientMessage) -> serde_json::Value {
        let text = serde_json::to_string(&msg).unwrap();
        assert_eq!(serde_json::from_str::<ClientMessage>(&text).unwrap(), msg);
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn item_messages_round_trip_with_documented_shape() {
        let v = round_trip_client(ClientMessage::ItemCreate {
            request_id: Some("r1".into()),
            id: "counter".into(),
            icon: None,
            label: Some("0".into()),
            tooltip: None,
        });
        assert_eq!(
            v,
            json!({"type":"item.create","requestId":"r1","id":"counter","label":"0"})
        );
        let v = round_trip_client(ClientMessage::ItemUpdate {
            request_id: None,
            id: "counter".into(),
            icon: None,
            label: Some(String::new()),
            tooltip: None,
            visible: Some(false),
        });
        assert_eq!(
            v,
            json!({"type":"item.update","id":"counter","label":"","visible":false})
        );
        let v = round_trip_client(ClientMessage::ItemRemove {
            request_id: None,
            id: "counter".into(),
        });
        assert_eq!(v, json!({"type":"item.remove","id":"counter"}));
        round_trip_client(ClientMessage::DebugListItems { request_id: None });
        let v = round_trip_client(ClientMessage::DebugClick {
            request_id: None,
            id: "a".into(),
            button: MouseButton::Middle,
        });
        assert_eq!(v["button"], "middle");
    }

    #[test]
    fn missing_optional_fields_parse_as_none() {
        let m: ClientMessage = serde_json::from_str(r#"{"type":"item.update","id":"a"}"#).unwrap();
        assert!(matches!(
            m,
            ClientMessage::ItemUpdate {
                icon: None,
                label: None,
                tooltip: None,
                visible: None,
                ..
            }
        ));
    }

    #[test]
    fn missing_required_fields_and_bad_button_fail() {
        assert!(serde_json::from_str::<ClientMessage>(r#"{"type":"item.create"}"#).is_err());
        assert!(serde_json::from_str::<ClientMessage>(
            r#"{"type":"debug.click","id":"a","button":"thumb"}"#
        )
        .is_err());
        assert!(serde_json::from_str::<ClientMessage>(r#"{"id":"a"}"#).is_err());
    }

    #[test]
    fn click_and_items_reply_round_trip() {
        let click = BarMessage::ItemClick {
            id: "a".into(),
            button: MouseButton::Left,
        };
        let v = serde_json::to_value(&click).unwrap();
        assert_eq!(v, json!({"type":"item.click","id":"a","button":"left"}));
        let items = BarMessage::DebugItems {
            request_id: Some("r".into()),
            items: vec![ItemInfo {
                id: "a".into(),
                app_name: "App".into(),
                pid: 1,
                has_icon: false,
                label: Some("1".into()),
                tooltip: None,
                visible: true,
            }],
        };
        let text = serde_json::to_string(&items).unwrap();
        assert_eq!(serde_json::from_str::<BarMessage>(&text).unwrap(), items);
    }

    #[test]
    fn request_id_is_readable_from_any_request() {
        let m: ClientMessage =
            serde_json::from_str(r#"{"type":"item.remove","id":"a","requestId":"x"}"#).unwrap();
        assert_eq!(m.request_id(), Some("x"));
    }

    #[test]
    fn client_message_types_list_matches_the_enum() {
        let samples = [
            r#"{"type":"hello","protocolVersion":"0.2","appName":"A","pid":1,"sdk":{"name":"x","version":"1"}}"#,
            r#"{"type":"item.create","id":"a"}"#,
            r#"{"type":"item.update","id":"a"}"#,
            r#"{"type":"item.remove","id":"a"}"#,
            r#"{"type":"debug.listItems"}"#,
            r#"{"type":"debug.click","id":"a","button":"left"}"#,
        ];
        for sample in samples {
            let msg: ClientMessage = serde_json::from_str(sample).unwrap();
            let value = serde_json::to_value(&msg).unwrap();
            assert!(
                CLIENT_MESSAGE_TYPES.contains(&value["type"].as_str().unwrap()),
                "{sample}"
            );
        }
        assert_eq!(samples.len(), CLIENT_MESSAGE_TYPES.len());
    }
}
