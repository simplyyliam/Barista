//! Things the control center can start: lock, sleep, restart, shut down, sign out, a screenshot,
//! and Windows' own settings pages. No state, so no snapshot or event.
//!
//! `decide` is the only logic and is unit tested. The executor below it really locks or shuts
//! down, so no test calls it.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Lock,
    Sleep,
    Restart,
    ShutDown,
    SignOut,
    Screenshot,
}

impl Action {
    /// Restart, shut down and sign out can lose work, so the UI must have asked the user first.
    pub fn is_destructive(self) -> bool {
        matches!(self, Self::Restart | Self::ShutDown | Self::SignOut)
    }
}

/// The settings pages the UI can open. An enum, never a free string, so the UI can't make us
/// launch an arbitrary URI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SettingsPage {
    Wifi,
    Bluetooth,
    Sound,
    Display,
    AirplaneMode,
    Hotspot,
    NightLight,
    BatterySaver,
    Focus,
    Accounts,
    Power,
    Location,
    /// Windows Settings' start page (the gear in the control center).
    Home,
}

impl SettingsPage {
    pub fn uri(self) -> &'static str {
        match self {
            Self::Wifi => "ms-settings:network-wifi",
            Self::Bluetooth => "ms-settings:bluetooth",
            Self::Sound => "ms-settings:sound",
            Self::Display => "ms-settings:display",
            Self::AirplaneMode => "ms-settings:network-airplanemode",
            Self::Hotspot => "ms-settings:network-mobilehotspot",
            Self::NightLight => "ms-settings:nightlight",
            Self::BatterySaver => "ms-settings:batterysaver",
            Self::Focus => "ms-settings:quiethours",
            Self::Accounts => "ms-settings:yourinfo",
            Self::Power => "ms-settings:powersleep",
            Self::Location => "ms-settings:privacy-location",
            Self::Home => "ms-settings:",
        }
    }
}

/// Apps a quick action can start. An enum with fixed URIs, so the UI can't launch anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum App {
    Calculator,
    ClockTimer,
    Camera,
}

impl App {
    pub fn uri(self) -> &'static str {
        match self {
            Self::Calculator => "calculator:",
            Self::ClockTimer => "ms-clock:",
            Self::Camera => "microsoft.windows.camera:",
        }
    }
}

pub fn open_app(app: App) {
    #[cfg(windows)]
    crate::platform::windows::shell::open_uri(app.uri());
    #[cfg(not(windows))]
    let _ = app;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ActionError {
    /// A destructive action came without `confirm: true`.
    NeedsConfirm,
}

/// Whether the action may run now.
pub fn decide(action: Action, confirm: bool) -> Result<Action, ActionError> {
    if action.is_destructive() && !confirm {
        Err(ActionError::NeedsConfirm)
    } else {
        Ok(action)
    }
}

pub fn run(action: Action, confirm: bool) -> Result<(), ActionError> {
    let action = decide(action, confirm)?;
    #[cfg(windows)]
    {
        use crate::platform::windows::shell;
        // Own thread: `SetSuspendState` only returns after the PC wakes, and a command runs on the
        // thread that also serves the bar's window.
        let spawned = std::thread::Builder::new().name("barista-action".into()).spawn(move || {
            match action {
                Action::Lock => shell::lock(),
                Action::Sleep => shell::sleep(),
                Action::Restart => shell::restart(),
                Action::ShutDown => shell::shut_down(),
                Action::SignOut => shell::sign_out(),
                Action::Screenshot => shell::open_uri("ms-screenclip:"),
            }
        });
        if let Err(e) = spawned {
            eprintln!("barista: could not run the action: {e}");
        }
    }
    #[cfg(not(windows))]
    let _ = action;
    Ok(())
}

pub fn open_settings(page: SettingsPage) {
    #[cfg(windows)]
    crate::platform::windows::shell::open_uri(page.uri());
    #[cfg(not(windows))]
    let _ = page;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn destructive_actions_need_confirm() {
        for a in [Action::Restart, Action::ShutDown, Action::SignOut] {
            assert_eq!(decide(a, false), Err(ActionError::NeedsConfirm));
            assert_eq!(decide(a, true), Ok(a));
        }
    }

    #[test]
    fn harmless_actions_do_not() {
        for a in [Action::Lock, Action::Sleep, Action::Screenshot] {
            assert_eq!(decide(a, false), Ok(a));
        }
    }

    #[test]
    fn every_page_is_a_settings_uri() {
        use SettingsPage::*;
        for p in [
            Wifi, Bluetooth, Sound, Display, AirplaneMode, Hotspot, NightLight, BatterySaver, Focus,
            Accounts, Power, Location, Home,
        ] {
            assert!(p.uri().starts_with("ms-settings:"));
        }
    }

    #[test]
    fn apps_have_fixed_uris() {
        for a in [App::Calculator, App::ClockTimer, App::Camera] {
            assert!(a.uri().ends_with(':'));
        }
    }
}
