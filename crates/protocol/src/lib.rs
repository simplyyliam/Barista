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
pub const PROTOCOL_VERSION: &str = "0.4";

/// Largest frame (the JSON part) either side accepts.
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;
/// Largest icon after base64 decoding (PNG or SVG).
pub const MAX_ICON_BYTES: usize = 256 * 1024;
/// Largest PNG width or height. A tiny file can declare a huge picture that costs gigabytes to decode.
pub const MAX_ICON_PIXELS: u32 = 1024;
pub const MAX_ID_LEN: usize = 64;
pub const MAX_LABEL_LEN: usize = 128;
pub const MAX_TOOLTIP_LEN: usize = 512;
pub const MAX_APP_NAME_LEN: usize = 64;
pub const MAX_ITEMS_PER_CLIENT: usize = 8;
/// A top-level menu is depth 1, its submenus 2, theirs 3.
pub const MAX_MENU_DEPTH: usize = 3;
/// Every entry in the whole tree counts, separators too.
pub const MAX_MENU_ITEMS: usize = 50;
pub const MAX_ACCELERATOR_LEN: usize = 32;
/// Menu icons are drawn at 16 px, so they stay small.
pub const MAX_MENU_ICON_BYTES: usize = 16 * 1024;
/// A popover is asked for in logical px: at least this wide and tall (`0x0` removes it)...
pub const MIN_POPOVER_SIZE: u32 = 48;
/// ...and at most this (the bar also clamps it to the work area).
pub const MAX_POPOVER_SIZE: u32 = 1024;

/// Every `type` a client may send. The bar uses it to tell `unknown_type` (a newer client)
/// from `invalid_message` (a known type with wrong fields). A test keeps it in sync.
pub const CLIENT_MESSAGE_TYPES: [&str; 11] = [
    "hello",
    "item.create",
    "item.update",
    "item.remove",
    "item.setMenu",
    "item.setPopover",
    "popover.shown",
    "popover.closed",
    "debug.listItems",
    "debug.click",
    "debug.menuClick",
];

/// What kind of menu entry. Electron's names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum MenuItemType {
    #[default]
    Normal,
    Checkbox,
    Radio,
    Separator,
    Submenu,
}

/// Standard entries the bar places and labels itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum MenuRole {
    /// The bar asks the app to quit (the SDK closes it unless the app handles the click).
    Quit,
    About,
    Settings,
}

/// One entry of a menu template, as the client sends it (Electron-style). Click handlers stay
/// in the SDK: only the `id` goes over the wire and comes back in `menu.click`.
/// `type` defaults to `submenu` when `submenu` is set, otherwise `normal`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MenuTemplateItem {
    /// Required on everything except separators; unique in the whole menu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<MenuItemType>,
    /// Only for `checkbox` and `radio`. The bar keeps it up to date after clicks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// Default `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Default `true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    /// Shown as text next to the label, like `Ctrl+S`. The bar does not register it as a hotkey.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accelerator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<MenuRole>,
    /// A small `data:` URL (PNG or SVG), drawn at 16 px.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submenu: Option<Vec<MenuTemplateItem>>,
}

impl MenuTemplateItem {
    /// The type with the `submenu` default applied.
    pub fn kind(&self) -> MenuItemType {
        match self.kind {
            Some(kind) => kind,
            None if self.submenu.is_some() => MenuItemType::Submenu,
            None => MenuItemType::Normal,
        }
    }
}

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

/// A rectangle on screen in physical pixels (virtual desktop coordinates, so a second monitor
/// to the left has negative `x`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// The bar's look, so a popover can match it. `dark` = the bar and its menus are dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum Tone {
    Light,
    Dark,
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
    /// Whether the item has a menu (since 0.3; older bars leave it out).
    #[serde(default)]
    pub has_menu: bool,
    /// Whether the item has a popover (since 0.4; older bars leave it out).
    #[serde(default)]
    pub has_popover: bool,
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
    /// Gives an item a menu (replacing any old one). An empty list removes the menu, so clicks
    /// go back to `item.click`. Since 0.3.
    #[serde(rename = "item.setMenu")]
    ItemSetMenu {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        id: String,
        menu: Vec<MenuTemplateItem>,
    },
    /// Gives an item a popover: the app's own window, shown under the icon on a left click.
    /// `width` and `height` are logical px (a request: the bar clamps to the work area);
    /// `0` x `0` removes the popover. Since 0.4.
    #[serde(rename = "item.setPopover")]
    ItemSetPopover {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        id: String,
        width: u32,
        height: u32,
    },
    /// The app showed its popover window after `popover.show` (clears the bar's timeout). Since 0.4.
    #[serde(rename = "popover.shown")]
    PopoverShown {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        id: String,
    },
    /// The popover window closed on the app's side (Esc, window closed). Since 0.4.
    #[serde(rename = "popover.closed")]
    PopoverClosed {
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
    /// Fakes a click on a menu entry of any item, as if the user picked it. `id` is the item's
    /// key (`s3/counter`). Same availability as `debug.listItems`.
    #[serde(rename = "debug.menuClick")]
    DebugMenuClick {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        id: String,
        menu_item_id: String,
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
            | Self::ItemSetMenu { request_id, .. }
            | Self::ItemSetPopover { request_id, .. }
            | Self::PopoverShown { request_id, .. }
            | Self::PopoverClosed { request_id, .. }
            | Self::DebugListItems { request_id }
            | Self::DebugClick { request_id, .. }
            | Self::DebugMenuClick { request_id, .. } => request_id.as_deref(),
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
    ItemClick {
        id: String,
        button: MouseButton,
        /// The item's rect on screen (since 0.4), for apps that place their own window.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor: Option<ScreenRect>,
    },
    /// Show your popover window exactly here: `rect` is already placed and clamped by the bar
    /// (physical px), `scale` is the monitor's DPI scale, `monitor` its id. Confirm with
    /// `popover.shown`. Sent only after a user click. Since 0.4.
    #[serde(rename = "popover.show")]
    PopoverShow {
        id: String,
        rect: ScreenRect,
        scale: f64,
        monitor: String,
        tone: Tone,
    },
    /// Hide your popover window (another item was clicked, the user clicked outside, or you
    /// took too long). Since 0.4.
    #[serde(rename = "popover.hide")]
    PopoverHide { id: String },
    /// The item's menu is about to show: refresh labels now. Since 0.4.
    #[serde(rename = "menu.opened")]
    MenuOpened { id: String },
    /// The item's menu closed. Since 0.4.
    #[serde(rename = "menu.closed")]
    MenuClosed { id: String },
    /// The user picked an entry in one of the client's menus. `id` is the item, `menu_item_id`
    /// the entry. For checkbox and radio entries, `checked` is the new state (the bar flips it).
    /// Since 0.3.
    #[serde(rename = "menu.click")]
    MenuClick {
        id: String,
        menu_item_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        checked: Option<bool>,
    },
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
    /// `item.setMenu` breaks a menu rule (depth, size, missing or repeated ids). Since 0.3.
    InvalidMenu,
    /// `item.setPopover` has a size outside the allowed range. Since 0.4.
    InvalidPopover,
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
                 export const MAX_ICON_PIXELS = {MAX_ICON_PIXELS}\n\
                 export const MAX_ID_LEN = {MAX_ID_LEN}\n\
                 export const MAX_LABEL_LEN = {MAX_LABEL_LEN}\n\
                 export const MAX_TOOLTIP_LEN = {MAX_TOOLTIP_LEN}\n\
                 export const MAX_APP_NAME_LEN = {MAX_APP_NAME_LEN}\n\
                 export const MAX_ITEMS_PER_CLIENT = {MAX_ITEMS_PER_CLIENT}\n\
                 export const MAX_MENU_DEPTH = {MAX_MENU_DEPTH}\n\
                 export const MAX_MENU_ITEMS = {MAX_MENU_ITEMS}\n\
                 export const MAX_ACCELERATOR_LEN = {MAX_ACCELERATOR_LEN}\n\
                 export const MAX_MENU_ICON_BYTES = {MAX_MENU_ICON_BYTES}\n\
                 export const MIN_POPOVER_SIZE = {MIN_POPOVER_SIZE}\n\
                 export const MAX_POPOVER_SIZE = {MAX_POPOVER_SIZE}\n"
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
            anchor: None,
        };
        let v = serde_json::to_value(&click).unwrap();
        assert_eq!(v, json!({"type":"item.click","id":"a","button":"left"}));
        let anchored = BarMessage::ItemClick {
            id: "a".into(),
            button: MouseButton::Left,
            anchor: Some(ScreenRect { x: -1920, y: 0, width: 24, height: 36 }),
        };
        let v = serde_json::to_value(&anchored).unwrap();
        assert_eq!(v["anchor"], json!({"x":-1920,"y":0,"width":24,"height":36}));
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
                has_menu: false,
                has_popover: false,
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
            r#"{"type":"item.setMenu","id":"a","menu":[]}"#,
            r#"{"type":"debug.menuClick","id":"s1/a","menuItemId":"b"}"#,
            r#"{"type":"item.setPopover","id":"a","width":320,"height":380}"#,
            r#"{"type":"popover.shown","id":"a"}"#,
            r#"{"type":"popover.closed","id":"a"}"#,
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

    #[test]
    fn popover_messages_have_the_documented_shape() {
        let show = BarMessage::PopoverShow {
            id: "note".into(),
            rect: ScreenRect { x: 100, y: 36, width: 480, height: 570 },
            scale: 1.5,
            monitor: "m1".into(),
            tone: Tone::Dark,
        };
        let v = serde_json::to_value(&show).unwrap();
        assert_eq!(
            v,
            json!({"type":"popover.show","id":"note","scale":1.5,"monitor":"m1","tone":"dark",
                   "rect":{"x":100,"y":36,"width":480,"height":570}})
        );
        for msg in [
            show,
            BarMessage::PopoverHide { id: "note".into() },
            BarMessage::MenuOpened { id: "note".into() },
            BarMessage::MenuClosed { id: "note".into() },
        ] {
            let text = serde_json::to_string(&msg).unwrap();
            assert_eq!(serde_json::from_str::<BarMessage>(&text).unwrap(), msg);
        }
    }

    /// A 0.3 client keeps working against a 0.4 bar: same major, nothing it sends changed.
    #[test]
    fn a_0_3_client_is_still_compatible() {
        assert!(is_compatible("0.3"));
        assert!(is_compatible("0.4"));
        assert!(!is_compatible("1.0"));
        // An item.click from a 0.3 bar (no anchor) still parses on a 0.4 client.
        let old: BarMessage =
            serde_json::from_str(r#"{"type":"item.click","id":"a","button":"left"}"#).unwrap();
        assert!(matches!(old, BarMessage::ItemClick { anchor: None, .. }));
    }

    #[test]
    fn menu_template_has_the_documented_shape() {
        let text = r#"{"type":"item.setMenu","id":"timer","menu":[
            {"id":"start","label":"Start","accelerator":"Ctrl+S"},
            {"id":"sound","label":"Sounds","type":"checkbox","checked":true},
            {"type":"separator"},
            {"id":"more","label":"More","submenu":[{"id":"about","label":"About","role":"about"}]},
            {"id":"quit","label":"Quit","role":"quit"}
        ]}"#;
        let msg: ClientMessage = serde_json::from_str(text).unwrap();
        let ClientMessage::ItemSetMenu { menu, .. } = &msg else {
            panic!("wrong variant")
        };
        assert_eq!(menu.len(), 5);
        assert_eq!(menu[0].kind(), MenuItemType::Normal);
        assert_eq!(menu[1].kind(), MenuItemType::Checkbox);
        assert_eq!(menu[2].kind(), MenuItemType::Separator);
        assert_eq!(menu[3].kind(), MenuItemType::Submenu);
        assert_eq!(menu[4].role, Some(MenuRole::Quit));
        let v = round_trip_client(msg);
        assert_eq!(v["menu"][1]["type"], "checkbox");
        assert!(v["menu"][0].get("type").is_none());
    }

    #[test]
    fn menu_click_round_trips() {
        let click = BarMessage::MenuClick {
            id: "timer".into(),
            menu_item_id: "sound".into(),
            checked: Some(false),
        };
        let v = serde_json::to_value(&click).unwrap();
        assert_eq!(
            v,
            json!({"type":"menu.click","id":"timer","menuItemId":"sound","checked":false})
        );
        assert_eq!(serde_json::from_value::<BarMessage>(v).unwrap(), click);
        let plain = serde_json::to_value(BarMessage::MenuClick {
            id: "t".into(),
            menu_item_id: "s".into(),
            checked: None,
        })
        .unwrap();
        assert!(plain.get("checked").is_none());
    }

    #[test]
    fn a_0_2_client_is_still_accepted() {
        // Backward compatible within major 0: a v0.2 SDK talks to this bar unchanged.
        assert!(is_compatible("0.2"));
        let old_items = r#"{"type":"debug.items","requestId":null,"items":[{"id":"s1/a",
            "appName":"A","pid":1,"hasIcon":false,"label":null,"tooltip":null,"visible":true}]}"#;
        assert!(serde_json::from_str::<BarMessage>(old_items).is_ok());
    }
}
