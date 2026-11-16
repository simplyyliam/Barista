//! Saved settings: bar height and monitor now; item order, pinned and hidden items later.
//! Stored as a small JSON file in the app's config folder.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// 28 like a notched MacBook's bar; 24 is Apple's HIG height without a notch. The UI's
// `--menubar-height` token (tokens.css) is the source of truth and calls `set_bar_height`.
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

/// How many ids the saved layout keeps and how long one may be (a hand-edited or runaway file must
/// not grow without bound, rule 3).
pub const MAX_LAYOUT_ENTRIES: usize = 512;
pub const MAX_LAYOUT_ID: usize = 300;

/// The user's arrangement of the bar's items (ADR 0028). Ids are stable across runs
/// (`app:<app>/<item>`, `tray:...`), not the per-run keys the UI sees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        let mut seen = std::collections::HashSet::new();
        self.order.retain(|id| ok(id) && seen.insert(id.clone()));
        self.order.truncate(MAX_LAYOUT_ENTRIES);
        self.pinned.retain(|id, _| ok(id));
        while self.pinned.len() > MAX_LAYOUT_ENTRIES {
            self.pinned.pop_last();
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Bar height in logical pixels (scaled by the monitor's DPI when applied).
    pub bar_height: u32,
    /// Id of the monitor the bar lives on. `None` = the primary monitor.
    pub monitor: Option<String>,
    /// Notification panel: ours or the Windows notification center.
    pub notifications_mode: Mode,
    /// Unix seconds when the notification panel was last opened (the unread dot).
    pub notifications_seen_at: u32,
    /// Control center (Wi-Fi, Bluetooth, brightness, ...): ours or Windows' Quick Settings.
    pub control_center_mode: Mode,
    /// Tray icons: the bar draws them (it hosts the tray window) or Windows keeps them (ADR 0026).
    pub tray_mode: Mode,
    /// How tray icons are laid out in the bar: a dropdown panel or unfolded inline.
    pub tray_style: TrayStyle,
    /// Order, pinned and collapsed state of the bar's items (v0.7).
    pub item_layout: ItemLayout,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            bar_height: DEFAULT_BAR_HEIGHT,
            monitor: None,
            notifications_mode: Mode::Barista,
            notifications_seen_at: 0,
            control_center_mode: Mode::Barista,
            tray_mode: Mode::Barista,
            tray_style: TrayStyle::Panel,
            item_layout: ItemLayout::default(),
        }
    }
}

impl Settings {
    /// A hand-edited file must not be able to make the bar huge or invisible.
    fn clamped(mut self) -> Self {
        self.bar_height = clamp_bar_height(self.bar_height);
        self.item_layout = self.item_layout.cleaned();
        self
    }

    /// Missing or broken file gives the defaults. Never fails: the bar must start.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
            .unwrap_or_default()
            .clamped()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("barista-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn missing_file_gives_defaults() {
        assert_eq!(
            Settings::load(&temp_file("missing.json")),
            Settings::default()
        );
    }

    #[test]
    fn broken_file_gives_defaults() {
        let path = temp_file("broken.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn height_is_clamped() {
        let path = temp_file("clamp.json");
        std::fs::write(&path, r#"{"barHeight": 5000}"#).unwrap();
        assert_eq!(Settings::load(&path).bar_height, MAX_BAR_HEIGHT);
        std::fs::write(&path, r#"{"barHeight": 0}"#).unwrap();
        assert_eq!(Settings::load(&path).bar_height, MIN_BAR_HEIGHT);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn default_height_is_28_and_clamp_keeps_it_in_range() {
        assert_eq!(Settings::default().bar_height, 28);
        assert_eq!(clamp_bar_height(28), 28);
        assert_eq!(clamp_bar_height(1), MIN_BAR_HEIGHT);
        assert_eq!(clamp_bar_height(900), MAX_BAR_HEIGHT);
    }

    #[test]
    fn old_file_without_notification_fields_keeps_the_defaults() {
        let path = temp_file("old.json");
        std::fs::write(&path, r#"{"barHeight":30}"#).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.bar_height, 30);
        assert_eq!(s.notifications_mode, Mode::Barista);
        assert_eq!(s.notifications_seen_at, 0);
        assert_eq!(s.control_center_mode, Mode::Barista);
        assert_eq!(s.tray_mode, Mode::Barista);
        assert_eq!(s.tray_style, TrayStyle::Panel);
        // No saved layout: nothing ordered or pinned, and the tray starts collapsed.
        assert_eq!(s.item_layout, ItemLayout::default());
        assert!(s.item_layout.collapsed);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_hand_edited_layout_is_cleaned_on_load() {
        let path = temp_file("layout.json");
        let long = "x".repeat(MAX_LAYOUT_ID + 1);
        let order: Vec<String> = (0..MAX_LAYOUT_ENTRIES + 50)
            .map(|i| format!("a{i}"))
            .chain(["".to_string(), long.clone(), "dup".to_string(), "dup".to_string()])
            .collect();
        let mut pinned = serde_json::Map::new();
        pinned.insert(String::new(), true.into());
        pinned.insert(long.clone(), true.into());
        pinned.insert("ok".into(), false.into());
        let text = serde_json::json!({ "itemLayout": { "order": order, "pinned": pinned } });
        std::fs::write(&path, text.to_string()).unwrap();
        let layout = Settings::load(&path).item_layout;
        assert_eq!(layout.order.len(), MAX_LAYOUT_ENTRIES);
        assert!(!layout.order.contains(&long) && !layout.order.iter().any(String::is_empty));
        assert_eq!(layout.pinned.len(), 1);
        assert_eq!(layout.pinned.get("ok"), Some(&false));
        // Duplicates collapse to one.
        let dup = ItemLayout { order: vec!["a".into(), "a".into(), "b".into()], ..Default::default() }.cleaned();
        assert_eq!(dup.order, ["a", "b"]);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = temp_file("round.json");
        let s = Settings {
            bar_height: 40,
            monitor: Some("monitor-a".into()),
            notifications_mode: Mode::Windows,
            notifications_seen_at: 1234,
            control_center_mode: Mode::Windows,
            tray_mode: Mode::Windows,
            tray_style: TrayStyle::Inline,
            item_layout: ItemLayout {
                order: vec!["tray:discord/1".into(), "app:Timer/main".into()],
                pinned: BTreeMap::from([("tray:discord/1".to_string(), true)]),
                collapsed: false,
            },
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        let _ = std::fs::remove_file(path);
    }
}
