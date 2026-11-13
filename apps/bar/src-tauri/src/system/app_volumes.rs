//! Per-app volume: one slider per app that has an audio session on the default output (the list
//! of Windows' volume mixer). Changes come from Windows events, never from polling; the list is
//! also read again when the volume panel opens. The audio thread is shared with `volume.rs`.

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppVolume {
    /// Stable per program (its file name, lower case). Pass it back to `setAppVolume` unchanged.
    pub id: String,
    /// What the program calls itself ("Google Chrome"), or its file name.
    pub name: String,
    /// The program's icon as a `data:image/png;base64,...` URL, when the shell has one.
    pub icon: Option<String>,
    /// 0.0 to 1.0.
    pub level: f64,
    pub muted: bool,
    /// Making sound right now. Apps that are only open come after the ones that play.
    pub playing: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppVolumes {
    pub apps: Vec<AppVolume>,
}

pub fn service() -> &'static Service<AppVolumes> {
    static S: OnceLock<Service<AppVolumes>> = OnceLock::new();
    S.get_or_init(|| Service::new(AppVolumes::default()))
}

#[cfg(windows)]
pub fn changed(raw: Vec<crate::platform::windows::audio_sessions::RawApp>) {
    super::publish_gated(
        service(),
        AppVolumes {
            apps: raw
                .into_iter()
                .map(|a| AppVolume {
                    id: a.id,
                    name: a.name,
                    icon: a.icon,
                    // Rounded like the master volume, so a slider fed by the event does not fight
                    // the value it just sent.
                    level: (f64::from(a.level) * 10_000.0).round() / 10_000.0,
                    muted: a.muted,
                    playing: a.playing,
                })
                .collect(),
        },
    );
}

/// Lists the apps again (the volume panel opened).
pub fn refresh() {
    #[cfg(windows)]
    crate::platform::windows::audio::refresh_apps();
}

/// Ids come from the list we published (a program file name): a sane length is checked here, and
/// the audio thread then only acts on an id it has in its own fresh list.
const MAX_ID: usize = 260;

/// An id from the UI that we are willing to pass on.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= MAX_ID && !id.chars().any(char::is_control)
}

pub fn set_volume(id: String, level: f64) {
    if !super::control_center_active() || !valid_id(&id) {
        return;
    }
    let Some(level) = super::volume::clean_level(level) else {
        return;
    };
    #[cfg(windows)]
    crate::platform::windows::audio::set_app_volume(id, level);
    #[cfg(not(windows))]
    let _ = (id, level);
}

pub fn set_muted(id: String, muted: bool) {
    if !super::control_center_active() || !valid_id(&id) {
        return;
    }
    #[cfg(windows)]
    crate::platform::windows::audio::set_app_muted(id, muted);
    #[cfg(not(windows))]
    let _ = (id, muted);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_must_be_short_and_printable() {
        assert!(valid_id("chrome.exe"));
        assert!(valid_id("pid:1234"));
        assert!(!valid_id(""));
        assert!(!valid_id("bad\nid"));
        assert!(!valid_id(&"a".repeat(MAX_ID + 1)));
        assert!(valid_id(&"a".repeat(MAX_ID)));
    }
}
