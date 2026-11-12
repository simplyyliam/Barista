//! Dark or light mode of Windows (apps and system together, like the Personalization page).
//! The registry has the truth; a watcher wakes us when anything changes it (also Windows' own
//! settings page), so nothing polls.

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ThemeMode {
    pub dark: bool,
}

pub fn service() -> &'static Service<ThemeMode> {
    static S: OnceLock<Service<ThemeMode>> = OnceLock::new();
    S.get_or_init(|| Service::new(ThemeMode::default()))
}

/// 0 = dark, 1 = light. A missing value means light (Windows' default).
pub fn from_registry(apps_use_light: Option<u32>) -> ThemeMode {
    ThemeMode {
        dark: apps_use_light == Some(0),
    }
}

#[cfg(windows)]
const KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

pub fn refresh() {
    #[cfg(windows)]
    service().set(from_registry(crate::platform::windows::registry_watch::read_user_dword(
        KEY,
        "AppsUseLightTheme",
    )));
}

pub fn set_dark(dark: bool) {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    {
        use crate::platform::windows::registry_watch::write_user_dword;
        let light = u32::from(!dark);
        write_user_dword(KEY, "AppsUseLightTheme", light);
        write_user_dword(KEY, "SystemUsesLightTheme", light);
        // The broadcast waits up to a second for every slow window, so not on the command thread.
        let _ = std::thread::Builder::new()
            .name("barista-theme-broadcast".into())
            .spawn(|| crate::platform::windows::shell::broadcast_setting_change("ImmersiveColorSet"));
    }
    #[cfg(not(windows))]
    let _ = dark;
}

pub fn start() {
    refresh();
    #[cfg(windows)]
    if !crate::platform::windows::registry_watch::watch_user(KEY, refresh) {
        eprintln!("barista: theme mode not watched");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_dark() {
        assert!(from_registry(Some(0)).dark);
        assert!(!from_registry(Some(1)).dark);
        assert!(!from_registry(None).dark);
    }
}
