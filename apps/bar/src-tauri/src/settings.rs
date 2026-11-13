//! Saved settings: bar height and monitor now; item order, pinned and hidden items later.
//! Stored as a small JSON file in the app's config folder.

use serde::{Deserialize, Serialize};
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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            bar_height: DEFAULT_BAR_HEIGHT,
            monitor: None,
            notifications_mode: Mode::Barista,
            notifications_seen_at: 0,
            control_center_mode: Mode::Barista,
        }
    }
}

impl Settings {
    /// A hand-edited file must not be able to make the bar huge or invisible.
    fn clamped(mut self) -> Self {
        self.bar_height = clamp_bar_height(self.bar_height);
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
        };
        s.save(&path).unwrap();
        assert_eq!(Settings::load(&path), s);
        let _ = std::fs::remove_file(path);
    }
}
