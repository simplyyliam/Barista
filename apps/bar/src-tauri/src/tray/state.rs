//! The table of tray icons we know about. Pure Rust (no Windows calls), so it is tested with
//! plain values. The worker feeds it parsed messages and turns the result into registry calls.

use super::parse::{Action, IconKey, TrayMessage};
use std::collections::HashMap;

/// Icons one process may have (rule 3: one noisy app must not fill the bar).
pub const MAX_PER_PROCESS: usize = 64;
/// Icons in total.
pub const MAX_TOTAL: usize = 256;

/// What a message does to the picture. The worker converts HICON to PNG before applying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Picture {
    /// The message carries no icon: keep what we have.
    Keep,
    /// The message clears the icon (a null handle) or the icon could not be converted.
    Clear,
    /// A PNG data URL.
    Set(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayIcon {
    pub pid: u32,
    pub app_name: String,
    /// The owner's window and id, and the callback message: v0.7 needs them to forward clicks.
    pub hwnd: u32,
    pub id: u32,
    pub callback_message: Option<u32>,
    pub version: u32,
    pub icon: Option<String>,
    pub tip: Option<String>,
    pub hidden: bool,
}

/// What a message did, so the caller can update the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Added or changed: show (or update) this icon.
    Upsert(IconKey),
    /// Gone.
    Remove(Vec<IconKey>),
    /// Nothing to do (unknown icon, focus request, or a limit was hit).
    Nothing,
}

#[derive(Default)]
pub struct TrayState {
    icons: HashMap<IconKey, TrayIcon>,
}

impl TrayState {
    pub fn get(&self, key: &IconKey) -> Option<&TrayIcon> {
        self.icons.get(key)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.icons.len()
    }

    /// Windows that still own at least one icon (so the host knows which to watch).
    pub fn owns_window(&self, hwnd: u32) -> bool {
        self.icons.values().any(|i| i.hwnd == hwnd)
    }

    /// Applies one message. `app_name` is only used when the message adds a new icon.
    pub fn apply(
        &mut self,
        message: &TrayMessage,
        pid: u32,
        app_name: &str,
        picture: Picture,
    ) -> Outcome {
        match message.action {
            Action::Add => self.add(message, pid, app_name, picture),
            Action::Modify => self.modify(message, picture),
            Action::Delete => match self.icons.remove(&message.key) {
                Some(_) => Outcome::Remove(vec![message.key]),
                None => Outcome::Nothing,
            },
            Action::SetVersion => {
                if let (Some(icon), Some(version)) =
                    (self.icons.get_mut(&message.key), message.version)
                {
                    icon.version = version;
                }
                Outcome::Nothing
            }
            Action::SetFocus => Outcome::Nothing,
        }
    }

    fn add(&mut self, message: &TrayMessage, pid: u32, app_name: &str, picture: Picture) -> Outcome {
        // An app that adds the same icon again (after `TaskbarCreated`) just refreshes it.
        if self.icons.contains_key(&message.key) {
            return self.modify(message, picture);
        }
        let from_process = self.icons.values().filter(|i| i.pid == pid).count();
        if from_process >= MAX_PER_PROCESS || self.icons.len() >= MAX_TOTAL {
            return Outcome::Nothing;
        }
        let mut icon = TrayIcon {
            pid,
            app_name: app_name.to_string(),
            hwnd: message.hwnd,
            id: message.id,
            callback_message: message.callback_message,
            version: 0,
            icon: None,
            tip: None,
            hidden: false,
        };
        change(&mut icon, message, picture);
        self.icons.insert(message.key, icon);
        Outcome::Upsert(message.key)
    }

    fn modify(&mut self, message: &TrayMessage, picture: Picture) -> Outcome {
        // Explorer ignores a modify for an icon it never saw, and so do we.
        let Some(icon) = self.icons.get_mut(&message.key) else {
            return Outcome::Nothing;
        };
        let before = icon.clone();
        change(icon, message, picture);
        if *icon == before {
            Outcome::Nothing
        } else {
            Outcome::Upsert(message.key)
        }
    }

    /// Removes every icon of a window that was destroyed.
    pub fn remove_window(&mut self, hwnd: u32) -> Outcome {
        let keys: Vec<IconKey> = self
            .icons
            .iter()
            .filter(|(_, i)| i.hwnd == hwnd)
            .map(|(k, _)| *k)
            .collect();
        for key in &keys {
            self.icons.remove(key);
        }
        if keys.is_empty() {
            Outcome::Nothing
        } else {
            Outcome::Remove(keys)
        }
    }
}

/// Copies the fields a message carries onto an icon.
fn change(icon: &mut TrayIcon, message: &TrayMessage, picture: Picture) {
    if let Some(callback) = message.callback_message {
        icon.callback_message = Some(callback);
    }
    match picture {
        Picture::Keep => {}
        Picture::Clear => icon.icon = None,
        Picture::Set(png) => icon.icon = Some(png),
    }
    if let Some(tip) = &message.tip {
        icon.tip = if tip.is_empty() { None } else { Some(tip.clone()) };
    }
    if let Some(hidden) = message.hidden {
        icon.hidden = hidden;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(action: Action, hwnd: u32, id: u32) -> TrayMessage {
        TrayMessage {
            action,
            key: IconKey::Handle { hwnd, id },
            hwnd,
            id,
            callback_message: None,
            icon_handle: None,
            tip: None,
            hidden: None,
            version: None,
        }
    }

    fn key(hwnd: u32, id: u32) -> IconKey {
        IconKey::Handle { hwnd, id }
    }

    fn png(text: &str) -> Picture {
        Picture::Set(text.to_string())
    }

    #[test]
    fn add_modify_delete() {
        let mut state = TrayState::default();
        let mut add = message(Action::Add, 1, 1);
        add.tip = Some("Discord".into());
        assert_eq!(
            state.apply(&add, 10, "Discord", png("a")),
            Outcome::Upsert(key(1, 1))
        );
        let icon = state.get(&key(1, 1)).unwrap();
        assert_eq!(icon.app_name, "Discord");
        assert_eq!(icon.tip.as_deref(), Some("Discord"));
        assert_eq!(icon.icon.as_deref(), Some("a"));

        let mut modify = message(Action::Modify, 1, 1);
        modify.tip = Some("Discord (3)".into());
        assert_eq!(
            state.apply(&modify, 10, "Discord", Picture::Keep),
            Outcome::Upsert(key(1, 1))
        );
        // The picture is kept when a modify only changes the tip.
        assert_eq!(state.get(&key(1, 1)).unwrap().icon.as_deref(), Some("a"));

        assert_eq!(
            state.apply(&message(Action::Delete, 1, 1), 10, "", Picture::Keep),
            Outcome::Remove(vec![key(1, 1)])
        );
        assert_eq!(state.len(), 0);
    }

    #[test]
    fn modify_or_delete_of_an_unknown_icon_does_nothing() {
        let mut state = TrayState::default();
        let mut modify = message(Action::Modify, 1, 1);
        modify.tip = Some("x".into());
        assert_eq!(state.apply(&modify, 1, "", Picture::Keep), Outcome::Nothing);
        assert_eq!(
            state.apply(&message(Action::Delete, 1, 1), 1, "", Picture::Keep),
            Outcome::Nothing
        );
        assert_eq!(state.len(), 0);
    }

    #[test]
    fn adding_twice_refreshes_instead_of_duplicating() {
        let mut state = TrayState::default();
        let mut add = message(Action::Add, 1, 1);
        add.tip = Some("one".into());
        state.apply(&add, 5, "App", png("a"));
        add.tip = Some("two".into());
        assert_eq!(state.apply(&add, 5, "App", png("a")), Outcome::Upsert(key(1, 1)));
        assert_eq!(state.len(), 1);
        assert_eq!(state.get(&key(1, 1)).unwrap().tip.as_deref(), Some("two"));
    }

    #[test]
    fn an_identical_modify_is_not_a_change() {
        // Some apps re-send the same tip every second (the system monitor here does).
        let mut state = TrayState::default();
        let mut add = message(Action::Add, 1, 1);
        add.tip = Some("same".into());
        state.apply(&add, 5, "App", png("a"));
        let mut modify = message(Action::Modify, 1, 1);
        modify.tip = Some("same".into());
        assert_eq!(state.apply(&modify, 5, "App", png("a")), Outcome::Nothing);
    }

    #[test]
    fn empty_tip_clears_it_and_null_icon_clears_the_picture() {
        let mut state = TrayState::default();
        let mut add = message(Action::Add, 1, 1);
        add.tip = Some("tip".into());
        state.apply(&add, 5, "App", png("a"));
        let mut modify = message(Action::Modify, 1, 1);
        modify.tip = Some(String::new());
        state.apply(&modify, 5, "App", Picture::Clear);
        let icon = state.get(&key(1, 1)).unwrap();
        assert_eq!(icon.tip, None);
        assert_eq!(icon.icon, None);
    }

    #[test]
    fn hidden_flag_follows_the_message() {
        let mut state = TrayState::default();
        state.apply(&message(Action::Add, 1, 1), 5, "App", Picture::Clear);
        let mut modify = message(Action::Modify, 1, 1);
        modify.hidden = Some(true);
        state.apply(&modify, 5, "App", Picture::Keep);
        assert!(state.get(&key(1, 1)).unwrap().hidden);
        modify.hidden = Some(false);
        state.apply(&modify, 5, "App", Picture::Keep);
        assert!(!state.get(&key(1, 1)).unwrap().hidden);
    }

    #[test]
    fn one_process_cannot_fill_the_bar() {
        let mut state = TrayState::default();
        for id in 0..MAX_PER_PROCESS as u32 {
            let outcome = state.apply(&message(Action::Add, 1, id), 7, "Noisy", Picture::Clear);
            assert!(matches!(outcome, Outcome::Upsert(_)));
        }
        let over = state.apply(&message(Action::Add, 1, 999), 7, "Noisy", Picture::Clear);
        assert_eq!(over, Outcome::Nothing);
        // Another process still gets in.
        let other = state.apply(&message(Action::Add, 2, 1), 8, "Quiet", Picture::Clear);
        assert!(matches!(other, Outcome::Upsert(_)));
    }

    #[test]
    fn total_limit() {
        let mut state = TrayState::default();
        for i in 0..MAX_TOTAL as u32 {
            // One icon per process, so only the total limit can stop us.
            state.apply(&message(Action::Add, i + 1, 1), i + 1, "A", Picture::Clear);
        }
        assert_eq!(state.len(), MAX_TOTAL);
        let over = state.apply(&message(Action::Add, 9999, 1), 9999, "A", Picture::Clear);
        assert_eq!(over, Outcome::Nothing);
    }

    #[test]
    fn a_destroyed_window_takes_all_its_icons() {
        let mut state = TrayState::default();
        state.apply(&message(Action::Add, 1, 1), 5, "App", Picture::Clear);
        state.apply(&message(Action::Add, 1, 2), 5, "App", Picture::Clear);
        state.apply(&message(Action::Add, 2, 1), 6, "Other", Picture::Clear);
        assert!(state.owns_window(1));
        match state.remove_window(1) {
            Outcome::Remove(keys) => assert_eq!(keys.len(), 2),
            other => panic!("unexpected {other:?}"),
        }
        assert!(!state.owns_window(1));
        assert_eq!(state.len(), 1);
        assert_eq!(state.remove_window(1), Outcome::Nothing);
    }

    #[test]
    fn set_version_is_remembered() {
        let mut state = TrayState::default();
        state.apply(&message(Action::Add, 1, 1), 5, "App", Picture::Clear);
        let mut version = message(Action::SetVersion, 1, 1);
        version.version = Some(4);
        assert_eq!(state.apply(&version, 5, "App", Picture::Keep), Outcome::Nothing);
        assert_eq!(state.get(&key(1, 1)).unwrap().version, 4);
    }
}
