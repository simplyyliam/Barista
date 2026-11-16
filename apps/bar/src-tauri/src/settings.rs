//! Saved settings, grouped like the sections of the settings window (ADR 0029).
//! Stored as a small JSON file in the app's config folder. Everything the user can change lives
//! here, validated and clamped on load and on every write (rule 3: a hand-edited file or an
//! imported one must not be able to break the bar).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

// 28 like a notched MacBook's bar; 24 is Apple's HIG height without a notch.
pub const DEFAULT_BAR_HEIGHT: u32 = 28;
pub const MIN_BAR_HEIGHT: u32 = 20;
pub const MAX_BAR_HEIGHT: u32 = 64;

pub fn clamp_bar_height(height: u32) -> u32 {
    height.clamp(MIN_BAR_HEIGHT, MAX_BAR_HEIGHT)
}

/// Who draws a feature that replaces a Windows one (CLAUDE.md rule 11). `Windows` means the bar
/// does nothing for it and opens the Windows version instead.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    Barista,
    Windows,
}

/// How the tray icons are shown in the bar (a preference, not a Windows replacement switch).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TrayStyle {
    /// An arrow in the bar opens a panel under it with the icons in a grid. Nothing in the bar moves.
    #[default]
    Panel,
    /// The icons unfold inside the bar, next to the arrow. Needs room: it can push other items off a
    /// narrow bar.
    Inline,
}

/// How many ids the saved layout keeps and how long one may be.
pub const MAX_LAYOUT_ENTRIES: usize = 512;
pub const MAX_LAYOUT_ID: usize = 300;

/// The user's arrangement of the bar's app and tray items (ADR 0028). Ids are stable across runs
/// (`app:<app>/<item>`, `tray:...`), not the per-run keys the UI sees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct ItemLayout {
    /// Left to right. Items not listed go after, in arrival order.
    pub order: Vec<String>,
    /// The user's pin choices. No entry = the default (client items pinned, tray icons not).
    pub pinned: BTreeMap<String, bool>,
    /// While collapsed, only pinned items stay in the bar; the rest go to the overflow list.
    pub collapsed: bool,
}

impl Default for ItemLayout {
    fn default() -> Self {
        Self {
            order: Vec::new(),
            pinned: BTreeMap::new(),
            collapsed: true,
        }
    }
}

impl ItemLayout {
    /// Drops empty or oversized ids and duplicates, and cuts the lists to their limits.
    pub fn cleaned(mut self) -> Self {
        let ok = |id: &String| !id.is_empty() && id.len() <= MAX_LAYOUT_ID;
        let mut seen = HashSet::new();
        self.order.retain(|id| ok(id) && seen.insert(id.clone()));
        self.order.truncate(MAX_LAYOUT_ENTRIES);
        self.pinned.retain(|id, _| ok(id));
        while self.pinned.len() > MAX_LAYOUT_ENTRIES {
            self.pinned.pop_last();
        }
        self
    }
}

// ---------------------------------------------------------------- General

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct General {
    /// Id of the monitor the bar lives on. `None` = the primary monitor.
    pub monitor: Option<String>,
    /// Mirror of the Run key (the registry is the truth, see `platform::windows::autostart`).
    pub start_with_windows: bool,
}

// ---------------------------------------------------------------- Menu bar

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ToneChoice {
    /// Follows the wallpaper behind the bar (ADR 0009).
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Blur {
    /// Blur when Windows has transparency on and no battery saver.
    #[default]
    Auto,
    /// Menus and panels use solid theme colours.
    Off,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum IconStyle {
    /// Apps' own colours.
    #[default]
    Colour,
    /// Every app and tray icon becomes a silhouette in the bar's tone.
    Mono,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum BarBackground {
    #[default]
    Translucent,
    Solid,
}

pub const MIN_FONT_SIZE: u8 = 11;
pub const MAX_FONT_SIZE: u8 = 16;
pub const MIN_OPACITY: u8 = 30;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub tone: ToneChoice,
    pub blur: Blur,
    /// A font family name installed on the PC. Empty = the bar's own default (Segoe UI Variable).
    /// Only letters, digits, spaces, dots, dashes and underscores (it ends up in CSS), 64 at most.
    pub font: String,
    /// Bar text size in logical px.
    pub font_size: u8,
    pub icon_style: IconStyle,
    pub background: BarBackground,
    /// Opacity of the bar's solid background in percent (only matters while `background` is solid).
    pub opacity: u8,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            tone: ToneChoice::Auto,
            blur: Blur::Auto,
            font: String::new(),
            font_size: 13,
            icon_style: IconStyle::Colour,
            background: BarBackground::Translucent,
            opacity: 100,
        }
    }
}

/// What a bar item shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ItemDisplay {
    Icon,
    Label,
    #[default]
    Both,
}

/// One of the bar's own items. The vector's order is the bar's order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct BarItemPref {
    pub id: String,
    pub visible: bool,
    pub display: ItemDisplay,
}

impl Default for BarItemPref {
    fn default() -> Self {
        Self {
            id: String::new(),
            visible: true,
            display: ItemDisplay::Both,
        }
    }
}

/// Built-in items in the default order (left group first, then the right group as it is drawn).
pub const BAR_ITEMS: [&str; 8] = [
    "brand",
    "app-name",
    "battery",
    "wifi",
    "volume",
    "search",
    "control-center",
    "clock",
];

fn default_bar_items() -> Vec<BarItemPref> {
    BAR_ITEMS
        .iter()
        .map(|id| BarItemPref {
            id: (*id).into(),
            ..Default::default()
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum HourFormat {
    /// What Windows uses.
    #[default]
    Auto,
    H12,
    H24,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DatePosition {
    #[default]
    Right,
    Center,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct ClockPrefs {
    pub hour_format: HourFormat,
    pub seconds: bool,
    pub weekday: bool,
    pub date: bool,
    pub date_position: DatePosition,
}

impl Default for ClockPrefs {
    fn default() -> Self {
        Self {
            hour_format: HourFormat::Auto,
            seconds: false,
            weekday: true,
            date: true,
            date_position: DatePosition::Right,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct Taskbar {
    /// Windows' own taskbar auto-hides while Barista runs (ADR 0031).
    pub auto_hide: bool,
    /// The taskbar state before we changed it, kept so a crash can be undone at the next start.
    pub saved_state: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct TrayPrefs {
    pub style: TrayStyle,
    /// Hide the icons Explorer itself puts in the tray (Safely Remove, Bluetooth, ...). Off by default:
    /// some of them are the only way to reach that feature.
    pub hide_windows_icons: bool,
}

impl Default for TrayPrefs {
    fn default() -> Self {
        Self {
            style: TrayStyle::Panel,
            hide_windows_icons: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct MenuBar {
    /// Logical px, scaled by the monitor's DPI when applied.
    pub height: u32,
    pub appearance: Appearance,
    pub items: Vec<BarItemPref>,
    pub clock: ClockPrefs,
    pub taskbar: Taskbar,
    pub tray: TrayPrefs,
    /// Order, pinned and collapsed state of app and tray items (v0.7).
    pub layout: ItemLayout,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self {
            height: DEFAULT_BAR_HEIGHT,
            appearance: Appearance::default(),
            items: default_bar_items(),
            clock: ClockPrefs::default(),
            taskbar: Taskbar::default(),
            tray: TrayPrefs::default(),
            layout: ItemLayout::default(),
        }
    }
}

// ---------------------------------------------------------------- Control center

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TileSize {
    #[default]
    Small,
    Wide,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct CcTile {
    pub id: String,
    pub size: TileSize,
    pub visible: bool,
}

impl Default for CcTile {
    fn default() -> Self {
        Self {
            id: String::new(),
            size: TileSize::Small,
            visible: true,
        }
    }
}

/// Toggle tiles, in the default order and size (the design: Wi-Fi and Battery Saver are wide).
pub const CC_TILES: [(&str, TileSize); 6] = [
    ("wifi", TileSize::Wide),
    ("airplane", TileSize::Small),
    ("night-light", TileSize::Small),
    ("bluetooth", TileSize::Small),
    ("hotspot", TileSize::Small),
    ("battery-saver", TileSize::Wide),
];

pub const QUICK_ACTION_IDS: [&str; 5] = ["dark-mode", "timer", "calculator", "camera", "screenshot"];
pub const MAX_QUICK_ACTIONS: usize = 8;

fn default_cc_tiles() -> Vec<CcTile> {
    CC_TILES
        .iter()
        .map(|(id, size)| CcTile {
            id: (*id).into(),
            size: *size,
            visible: true,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct ControlCenterPrefs {
    pub tiles: Vec<CcTile>,
    pub user_card: bool,
    pub brightness: bool,
    pub volume: bool,
    pub now_playing: bool,
    /// Ids from `QUICK_ACTION_IDS`, in order, at most 8. Empty hides the card.
    pub quick_actions: Vec<String>,
}

impl Default for ControlCenterPrefs {
    fn default() -> Self {
        Self {
            tiles: default_cc_tiles(),
            user_card: true,
            brightness: true,
            volume: true,
            now_playing: true,
            quick_actions: QUICK_ACTION_IDS.iter().map(|s| (*s).into()).collect(),
        }
    }
}

// ---------------------------------------------------------------- Behavior, Advanced

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct Behavior {
    /// Hide the bar while a full-screen app has focus (v0.1).
    pub hide_on_fullscreen: bool,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            hide_on_fullscreen: true,
        }
    }
}

/// The `barista | windows` switches (rule 11).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct Advanced {
    /// Notification panel: ours or the Windows notification center.
    pub notifications: Mode,
    /// Control center (Wi-Fi, Bluetooth, brightness, ...): ours or Windows' Quick Settings.
    pub control_center: Mode,
    /// Tray icons: the bar hosts the tray window or Windows keeps them (ADR 0026).
    pub tray: Mode,
}

// ---------------------------------------------------------------- Root

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub general: General,
    pub menu_bar: MenuBar,
    pub control_center: ControlCenterPrefs,
    pub behavior: Behavior,
    pub advanced: Advanced,
    /// Unix seconds when the notification panel was last opened (the unread dot). Not a preference.
    pub notifications_seen_at: u32,
}

/// The parts of the settings the reset command and the editor talk about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum Section {
    General,
    MenuBar,
    Appearance,
    Clock,
    ControlCenter,
    Behavior,
    Advanced,
    All,
}

impl Settings {
    /// A hand-edited or imported file must not be able to make the bar huge, invisible or unbounded.
    pub fn clamped(mut self) -> Self {
        let bar = &mut self.menu_bar;
        bar.height = clamp_bar_height(bar.height);
        bar.layout = std::mem::take(&mut bar.layout).cleaned();
        bar.appearance.font_size = bar.appearance.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        bar.appearance.opacity = bar.appearance.opacity.clamp(MIN_OPACITY, 100);
        bar.appearance.font = clean_font(std::mem::take(&mut bar.appearance.font));
        bar.items = clean_bar_items(std::mem::take(&mut bar.items));
        self.control_center.tiles = clean_tiles(std::mem::take(&mut self.control_center.tiles));
        self.control_center.quick_actions =
            clean_quick_actions(std::mem::take(&mut self.control_center.quick_actions));
        self
    }

    /// Puts one section back to its defaults. The notification stamp and the saved taskbar
    /// state are bookkeeping, not preferences: a reset never touches them.
    pub fn reset(&mut self, section: Section) {
        let keep_taskbar = self.menu_bar.taskbar.clone();
        let layout = self.menu_bar.layout.clone();
        match section {
            Section::General => {
                // Start with Windows is applied through the registry by the caller.
                self.general = General::default();
            }
            Section::MenuBar => {
                self.menu_bar = MenuBar::default();
                self.menu_bar.layout = layout;
            }
            Section::Appearance => self.menu_bar.appearance = Appearance::default(),
            Section::Clock => self.menu_bar.clock = ClockPrefs::default(),
            Section::ControlCenter => self.control_center = ControlCenterPrefs::default(),
            Section::Behavior => self.behavior = Behavior::default(),
            Section::Advanced => self.advanced = Advanced::default(),
            Section::All => {
                let seen = self.notifications_seen_at;
                *self = Settings::default();
                self.notifications_seen_at = seen;
            }
        }
        self.menu_bar.taskbar.saved_state = keep_taskbar.saved_state;
    }

    /// Missing or broken file gives the defaults. Never fails: the bar must start.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| Self::from_json(&text))
            .unwrap_or_default()
    }

    /// Reads settings JSON, including files from before v0.8 (flat keys). `None` if it is not
    /// settings JSON at all.
    pub fn from_json(text: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(text).ok()?;
        let value = migrate(value)?;
        serde_json::from_value::<Settings>(value)
            .ok()
            .map(Settings::clamped)
    }

    /// Writes to a temp file and renames it over the real one, so a crash mid-write
    /// can't leave a half-written settings file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, path)
    }
}

pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

// ---------------------------------------------------------------- cleaning

/// A font name goes into CSS, so only safe characters survive; anything else means "default".
pub const MAX_FONT_LEN: usize = 64;

pub fn clean_font(font: String) -> String {
    let font = font.trim().to_string();
    let ok = font.len() <= MAX_FONT_LEN
        && font
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '-' | '_'));
    if ok {
        font
    } else {
        String::new()
    }
}

/// Known ids only, no duplicates, missing ones appended at the end with their defaults (a new
/// item shows up for people who saved their bar before it existed).
fn clean_bar_items(items: Vec<BarItemPref>) -> Vec<BarItemPref> {
    let mut seen = HashSet::new();
    let mut out: Vec<BarItemPref> = items
        .into_iter()
        .filter(|i| BAR_ITEMS.contains(&i.id.as_str()) && seen.insert(i.id.clone()))
        .collect();
    for id in BAR_ITEMS {
        if !seen.contains(id) {
            out.push(BarItemPref {
                id: id.into(),
                ..Default::default()
            });
        }
    }
    out
}

fn clean_tiles(tiles: Vec<CcTile>) -> Vec<CcTile> {
    let mut seen = HashSet::new();
    let mut out: Vec<CcTile> = tiles
        .into_iter()
        .filter(|t| CC_TILES.iter().any(|(id, _)| *id == t.id) && seen.insert(t.id.clone()))
        .collect();
    for (id, size) in CC_TILES {
        if !seen.contains(id) {
            out.push(CcTile {
                id: id.into(),
                size,
                visible: true,
            });
        }
    }
    out
}

fn clean_quick_actions(actions: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    actions
        .into_iter()
        .filter(|a| QUICK_ACTION_IDS.contains(&a.as_str()) && seen.insert(a.clone()))
        .take(MAX_QUICK_ACTIONS)
        .collect()
}

// ---------------------------------------------------------------- migration

/// v0.7 and older kept flat keys (`barHeight`, `monitor`, `trayMode`, ...). Moves them into their
/// sections; a file that already has sections passes through. `None` for anything that is not an object.
fn migrate(value: Value) -> Option<Value> {
    let Value::Object(mut old) = value else {
        return None;
    };
    let legacy = [
        "barHeight",
        "monitor",
        "notificationsMode",
        "controlCenterMode",
        "trayMode",
        "trayStyle",
        "itemLayout",
    ];
    if !legacy.iter().any(|k| old.contains_key(*k)) {
        return Some(Value::Object(old));
    }
    let mut take = |key: &str| old.remove(key);
    let mut general = Map::new();
    let mut bar = Map::new();
    let mut tray = Map::new();
    let mut advanced = Map::new();
    if let Some(v) = take("monitor") {
        general.insert("monitor".into(), v);
    }
    if let Some(v) = take("barHeight") {
        bar.insert("height".into(), v);
    }
    if let Some(v) = take("itemLayout") {
        bar.insert("layout".into(), v);
    }
    if let Some(v) = take("trayStyle") {
        tray.insert("style".into(), v);
    }
    if let Some(v) = take("notificationsMode") {
        advanced.insert("notifications".into(), v);
    }
    if let Some(v) = take("controlCenterMode") {
        advanced.insert("controlCenter".into(), v);
    }
    if let Some(v) = take("trayMode") {
        advanced.insert("tray".into(), v);
    }
    if !tray.is_empty() {
        bar.insert("tray".into(), Value::Object(tray));
    }
    let mut out = Map::new();
    if let Some(v) = take("notificationsSeenAt") {
        out.insert("notificationsSeenAt".into(), v);
    }
    out.insert("general".into(), Value::Object(general));
    out.insert("menuBar".into(), Value::Object(bar));
    out.insert("advanced".into(), Value::Object(advanced));
    Some(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("barista-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn missing_file_gives_defaults() {
        assert_eq!(Settings::load(&temp_file("missing.json")), Settings::default());
    }

    #[test]
    fn broken_file_gives_defaults() {
        let path = temp_file("broken.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        std::fs::write(&path, "[1,2]").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn height_is_clamped() {
        let s = Settings::from_json(r#"{"menuBar":{"height": 5000}}"#).unwrap();
        assert_eq!(s.menu_bar.height, MAX_BAR_HEIGHT);
        let s = Settings::from_json(r#"{"menuBar":{"height": 0}}"#).unwrap();
        assert_eq!(s.menu_bar.height, MIN_BAR_HEIGHT);
        assert_eq!(Settings::default().menu_bar.height, 28);
    }

    #[test]
    fn a_v07_flat_file_moves_into_sections() {
        let text = r#"{"barHeight":30,"monitor":"m1","notificationsMode":"windows",
            "notificationsSeenAt":99,"controlCenterMode":"windows","trayMode":"windows",
            "trayStyle":"inline","itemLayout":{"order":["a"],"pinned":{"a":true},"collapsed":false}}"#;
        let s = Settings::from_json(text).unwrap();
        assert_eq!(s.menu_bar.height, 30);
        assert_eq!(s.general.monitor.as_deref(), Some("m1"));
        assert_eq!(s.advanced.notifications, Mode::Windows);
        assert_eq!(s.advanced.control_center, Mode::Windows);
        assert_eq!(s.advanced.tray, Mode::Windows);
        assert_eq!(s.menu_bar.tray.style, TrayStyle::Inline);
        assert_eq!(s.menu_bar.layout.order, ["a"]);
        assert!(!s.menu_bar.layout.collapsed);
        assert_eq!(s.notifications_seen_at, 99);
        // New sections start at their defaults.
        assert_eq!(s.control_center, ControlCenterPrefs::default());
    }

    #[test]
    fn old_file_with_only_a_height_keeps_the_defaults() {
        let s = Settings::from_json(r#"{"barHeight":30}"#).unwrap();
        assert_eq!(s.menu_bar.height, 30);
        assert_eq!(s.advanced, Advanced::default());
        assert_eq!(s.menu_bar.layout, ItemLayout::default());
        assert!(s.menu_bar.layout.collapsed);
    }

    #[test]
    fn a_hand_edited_layout_is_cleaned_on_load() {
        let long = "x".repeat(MAX_LAYOUT_ID + 1);
        let order: Vec<String> = (0..MAX_LAYOUT_ENTRIES + 50)
            .map(|i| format!("a{i}"))
            .chain(["".to_string(), long.clone(), "dup".to_string(), "dup".to_string()])
            .collect();
        let mut pinned = serde_json::Map::new();
        pinned.insert(String::new(), true.into());
        pinned.insert(long.clone(), true.into());
        pinned.insert("ok".into(), false.into());
        let text = serde_json::json!({ "menuBar": { "layout": { "order": order, "pinned": pinned } } });
        let layout = Settings::from_json(&text.to_string()).unwrap().menu_bar.layout;
        assert_eq!(layout.order.len(), MAX_LAYOUT_ENTRIES);
        assert!(!layout.order.contains(&long) && !layout.order.iter().any(String::is_empty));
        assert_eq!(layout.pinned.len(), 1);
        assert_eq!(layout.pinned.get("ok"), Some(&false));
        let dup = ItemLayout { order: vec!["a".into(), "a".into(), "b".into()], ..Default::default() }.cleaned();
        assert_eq!(dup.order, ["a", "b"]);
    }

    #[test]
    fn bar_items_drop_unknown_and_duplicates_and_gain_missing_ones() {
        let text = r#"{"menuBar":{"items":[
            {"id":"clock","visible":false},{"id":"clock"},{"id":"evil"},{"id":"wifi","display":"icon"}]}}"#;
        let items = Settings::from_json(text).unwrap().menu_bar.items;
        let ids: Vec<_> = items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(&ids[..2], ["clock", "wifi"]);
        assert_eq!(items.len(), BAR_ITEMS.len());
        assert!(!items[0].visible);
        assert_eq!(items[1].display, ItemDisplay::Icon);
        assert!(!ids.contains(&"evil"));
    }

    #[test]
    fn control_center_lists_are_cleaned_and_capped() {
        let many: Vec<&str> = std::iter::repeat("timer").take(20).collect();
        let text = serde_json::json!({ "controlCenter": {
            "quickActions": many, "tiles": [{"id":"bluetooth","size":"wide"},{"id":"nope"}] } });
        let s = Settings::from_json(&text.to_string()).unwrap();
        assert_eq!(s.control_center.quick_actions, ["timer"]);
        assert_eq!(s.control_center.tiles.len(), CC_TILES.len());
        assert_eq!(s.control_center.tiles[0].id, "bluetooth");
        assert_eq!(s.control_center.tiles[0].size, TileSize::Wide);
        let all: Vec<String> = QUICK_ACTION_IDS.iter().map(|s| (*s).into()).collect();
        assert_eq!(clean_quick_actions(all.clone()), all);
    }

    #[test]
    fn appearance_numbers_are_clamped() {
        let s = Settings::from_json(r#"{"menuBar":{"appearance":{"fontSize":200,"opacity":0}}}"#).unwrap();
        assert_eq!(s.menu_bar.appearance.font_size, MAX_FONT_SIZE);
        assert_eq!(s.menu_bar.appearance.opacity, MIN_OPACITY);
    }

    #[test]
    fn a_font_name_cannot_carry_css() {
        assert_eq!(clean_font("Cascadia Code".into()), "Cascadia Code");
        assert_eq!(clean_font("  Inter ".into()), "Inter");
        assert_eq!(clean_font("x; background:url(//evil)".into()), "");
        assert_eq!(clean_font("a\"b".into()), "");
        assert_eq!(clean_font("x".repeat(MAX_FONT_LEN + 1)), "");
        let s = Settings::from_json(r#"{"menuBar":{"appearance":{"font":"bad{font}"}}}"#).unwrap();
        assert_eq!(s.menu_bar.appearance.font, "");
    }

    #[test]
    fn reset_keeps_bookkeeping() {
        let mut s = Settings { notifications_seen_at: 5, ..Default::default() };
        s.menu_bar.taskbar.saved_state = Some(2);
        s.menu_bar.taskbar.auto_hide = true;
        s.menu_bar.appearance.font_size = 16;
        s.menu_bar.layout.order = vec!["a".into()];
        s.reset(Section::Appearance);
        assert_eq!(s.menu_bar.appearance, Appearance::default());
        s.reset(Section::MenuBar);
        assert_eq!(s.menu_bar.layout.order, ["a"]);
        s.reset(Section::All);
        assert_eq!(s.notifications_seen_at, 5);
        assert_eq!(s.menu_bar.taskbar.saved_state, Some(2));
        assert!(!s.menu_bar.taskbar.auto_hide);
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = temp_file("round.json");
        let mut s = Settings::default();
        s.general.monitor = Some("monitor-a".into());
        s.menu_bar.height = 40;
        s.menu_bar.tray.style = TrayStyle::Inline;
        s.menu_bar.clock.seconds = true;
        s.advanced.notifications = Mode::Windows;
        s.notifications_seen_at = 1234;
        s.menu_bar.layout = ItemLayout {
            order: vec!["tray:discord/1".into(), "app:Timer/main".into()],
            pinned: BTreeMap::from([("tray:discord/1".to_string(), true)]),
            collapsed: false,
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        let _ = std::fs::remove_file(path);
    }
}
