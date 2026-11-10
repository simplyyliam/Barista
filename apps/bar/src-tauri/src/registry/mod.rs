//! The item registry: every status item and every connected client session. The one place that
//! knows what is in the bar; the UI renders what `snapshot()` returns.
//!
//! Items are keyed by (session, id), so two apps can both call their item "main". The UI gets a
//! single string key, `"<session>/<id>"` (for example `"s3/counter"`). Item ids can't contain `/`
//! (see `ipc::validate`), so that key is unambiguous.

pub mod anchor;
pub mod menu;

use barista_protocol::{
    BarMessage, ErrorCode, ItemInfo, MenuTemplateItem, MouseButton, MAX_ITEMS_PER_CLIENT,
};
use menu::{MenuClickError, MenuEntry};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use tokio::sync::{mpsc, Notify};

/// A client connection. Printed as `s1`, `s2`, ...
pub type SessionId = u64;

/// How a session's reply queue is handed to the registry. Bounded on purpose: a client that
/// stops reading must never make the bar wait (CLAUDE.md rule 3).
pub type Outbox = mpsc::Sender<BarMessage>;

/// An item as the UI draws it. `id` is the joined `"<session>/<id>"` key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StatusItemView {
    pub id: String,
    pub app_name: String,
    /// A `data:` URL, ready for `<img src>`.
    pub icon: Option<String>,
    pub label: Option<String>,
    pub tooltip: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RegistryError {
    DuplicateId,
    NotFound,
    LimitReached,
}

impl RegistryError {
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::DuplicateId => ErrorCode::DuplicateId,
            Self::NotFound => ErrorCode::NotFound,
            Self::LimitReached => ErrorCode::LimitReached,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClickError {
    /// No such item (it may have just been removed).
    NotFound,
    /// The client's queue is full or its connection is gone.
    Unreachable,
}

/// What `update` changes. `None` = leave as is, `Some("")` = clear.
#[derive(Debug, Default)]
pub struct ItemPatch {
    pub icon: Option<String>,
    pub label: Option<String>,
    pub tooltip: Option<String>,
    pub visible: Option<bool>,
}

struct Session {
    app_name: String,
    pid: u32,
    outbox: Outbox,
}

struct Item {
    session: SessionId,
    id: String,
    icon: Option<String>,
    label: Option<String>,
    tooltip: Option<String>,
    visible: bool,
    /// Empty = no menu (clicks go to the app as `item.click`).
    menu: Vec<MenuTemplateItem>,
    /// Goes up on every menu change (new menu or a checked flip), so the open menu can tell
    /// when it must redraw.
    menu_rev: u64,
}

/// What an open menu needs: what to draw and a revision to spot changes.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuSnapshot {
    pub entries: Vec<MenuEntry>,
    pub rev: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MenuItemClickError {
    /// No such item, or it has no menu.
    NotFound,
    Menu(MenuClickError),
    /// The click was applied but the client's queue is full or gone.
    Unreachable,
}

#[derive(Default)]
struct State {
    next_session: SessionId,
    sessions: HashMap<SessionId, Session>,
    /// Arrival order. Drag ordering comes in v0.7.
    items: Vec<Item>,
}

#[derive(Default)]
pub struct Registry {
    // A plain std Mutex is fine: it is only held for quick map edits and never across an
    // `.await`, so it can't block the async runtime for long.
    state: Mutex<State>,
    /// Pinged after every change. `notify_one` keeps one permit if nobody is waiting yet, so
    /// a burst of changes becomes a single wake-up for the event emitter.
    changed: Notify,
}

/// `"s3"` -> 3
fn parse_session(text: &str) -> Option<SessionId> {
    text.strip_prefix('s')?.parse().ok()
}

/// `"s3/counter"` -> (3, "counter")
fn parse_key(key: &str) -> Option<(SessionId, &str)> {
    let (session, id) = key.split_once('/')?;
    Some((parse_session(session)?, id))
}

impl State {
    fn item_mut(&mut self, key: &str) -> Option<&mut Item> {
        let (session, id) = parse_key(key)?;
        self.items
            .iter_mut()
            .find(|i| i.session == session && i.id == id)
    }
}

fn clear_if_empty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // A panic while the lock was held leaves the data usable, so ignore the poison flag.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Resolves when something changed since the last wait.
    pub async fn wait_changed(&self) {
        self.changed.notified().await;
    }

    #[cfg(test)]
    pub fn session_count(&self) -> usize {
        self.lock().sessions.len()
    }

    pub fn add_session(&self, app_name: String, pid: u32, outbox: Outbox) -> SessionId {
        let mut state = self.lock();
        state.next_session += 1;
        let id = state.next_session;
        state.sessions.insert(
            id,
            Session {
                app_name,
                pid,
                outbox,
            },
        );
        id
    }

    /// Drops the session and every item it owned (client quit or crashed).
    pub fn remove_session(&self, session: SessionId) {
        let mut state = self.lock();
        let had_items = state.items.iter().any(|i| i.session == session);
        state.items.retain(|i| i.session != session);
        state.sessions.remove(&session);
        drop(state);
        if had_items {
            self.changed.notify_one();
        }
    }

    pub fn create(
        &self,
        session: SessionId,
        id: String,
        icon: Option<String>,
        label: Option<String>,
        tooltip: Option<String>,
    ) -> Result<(), RegistryError> {
        let mut state = self.lock();
        if !state.sessions.contains_key(&session) {
            return Err(RegistryError::NotFound);
        }
        let mine = state.items.iter().filter(|i| i.session == session);
        let (mut count, mut duplicate) = (0, false);
        for item in mine {
            count += 1;
            duplicate |= item.id == id;
        }
        if duplicate {
            return Err(RegistryError::DuplicateId);
        }
        if count >= MAX_ITEMS_PER_CLIENT {
            return Err(RegistryError::LimitReached);
        }
        state.items.push(Item {
            session,
            id,
            icon: icon.and_then(clear_if_empty),
            label: label.and_then(clear_if_empty),
            tooltip: tooltip.and_then(clear_if_empty),
            visible: true,
            menu: Vec::new(),
            menu_rev: 0,
        });
        drop(state);
        self.changed.notify_one();
        Ok(())
    }

    pub fn update(
        &self,
        session: SessionId,
        id: &str,
        patch: ItemPatch,
    ) -> Result<(), RegistryError> {
        let mut state = self.lock();
        let item = state
            .items
            .iter_mut()
            .find(|i| i.session == session && i.id == id)
            .ok_or(RegistryError::NotFound)?;
        let mut changed = false;
        // Only a real change wakes the UI, so a client that re-sends the same label is free.
        let mut set = |slot: &mut Option<String>, new: Option<String>| {
            if let Some(new) = new {
                let new = clear_if_empty(new);
                if *slot != new {
                    *slot = new;
                    changed = true;
                }
            }
        };
        set(&mut item.icon, patch.icon);
        set(&mut item.label, patch.label);
        set(&mut item.tooltip, patch.tooltip);
        if let Some(visible) = patch.visible {
            if item.visible != visible {
                item.visible = visible;
                changed = true;
            }
        }
        drop(state);
        if changed {
            self.changed.notify_one();
        }
        Ok(())
    }

    pub fn remove(&self, session: SessionId, id: &str) -> Result<(), RegistryError> {
        let mut state = self.lock();
        let before = state.items.len();
        state
            .items
            .retain(|i| !(i.session == session && i.id == id));
        let removed = state.items.len() != before;
        drop(state);
        if !removed {
            return Err(RegistryError::NotFound);
        }
        self.changed.notify_one();
        Ok(())
    }

    /// The visible items, in arrival order, as the UI draws them.
    pub fn snapshot(&self) -> Vec<StatusItemView> {
        let state = self.lock();
        state
            .items
            .iter()
            .filter(|i| i.visible)
            .filter_map(|i| {
                let session = state.sessions.get(&i.session)?;
                Some(StatusItemView {
                    id: format!("s{}/{}", i.session, i.id),
                    app_name: session.app_name.clone(),
                    icon: i.icon.clone(),
                    label: i.label.clone(),
                    tooltip: i.tooltip.clone(),
                })
            })
            .collect()
    }

    /// Every item including hidden ones, with its owner's pid. For `debug.listItems`.
    pub fn debug_items(&self) -> Vec<ItemInfo> {
        let state = self.lock();
        state
            .items
            .iter()
            .filter_map(|i| {
                let session = state.sessions.get(&i.session)?;
                Some(ItemInfo {
                    id: format!("s{}/{}", i.session, i.id),
                    app_name: session.app_name.clone(),
                    pid: session.pid,
                    has_icon: i.icon.is_some(),
                    label: i.label.clone(),
                    tooltip: i.tooltip.clone(),
                    visible: i.visible,
                    has_menu: !i.menu.is_empty(),
                })
            })
            .collect()
    }

    /// Gives an item a menu (already validated). An empty list removes it.
    pub fn set_menu(
        &self,
        session: SessionId,
        id: &str,
        menu: Vec<MenuTemplateItem>,
    ) -> Result<(), RegistryError> {
        let mut state = self.lock();
        let item = state
            .items
            .iter_mut()
            .find(|i| i.session == session && i.id == id)
            .ok_or(RegistryError::NotFound)?;
        let menu = menu::arrange(menu);
        if item.menu == menu {
            return Ok(());
        }
        item.menu = menu;
        item.menu_rev += 1;
        drop(state);
        self.changed.notify_one();
        Ok(())
    }

    /// Whether a click with `button` on `key` should open its menu instead of reaching the app.
    /// Middle clicks still go to the app.
    pub fn opens_menu(&self, key: &str, button: MouseButton) -> bool {
        button != MouseButton::Middle && self.menu(key).is_some()
    }

    /// The menu of a visible item, ready to draw. `None` if the item is gone, hidden or has no menu.
    pub fn menu(&self, key: &str) -> Option<MenuSnapshot> {
        let mut state = self.lock();
        let item = state.item_mut(key)?;
        if item.menu.is_empty() || !item.visible {
            return None;
        }
        let (session, menu, rev) = (item.session, item.menu.clone(), item.menu_rev);
        let app_name = &state.sessions.get(&session)?.app_name;
        Some(MenuSnapshot {
            entries: menu::view(&menu, app_name),
            rev,
        })
    }

    /// The user picked entry `menu_item_id` in the menu of `key`. Flips checkbox/radio state
    /// here, then sends `menu.click` (with the new `checked`) to the app.
    pub fn menu_click(&self, key: &str, menu_item_id: &str) -> Result<(), MenuItemClickError> {
        let mut state = self.lock();
        let item = state.item_mut(key).ok_or(MenuItemClickError::NotFound)?;
        if item.menu.is_empty() {
            return Err(MenuItemClickError::NotFound);
        }
        let checked =
            menu::click(&mut item.menu, menu_item_id).map_err(MenuItemClickError::Menu)?;
        if checked.is_some() {
            item.menu_rev += 1;
        }
        let (session, id) = (item.session, item.id.clone());
        let sent = state.sessions.get(&session).is_some_and(|s| {
            s.outbox
                .try_send(BarMessage::MenuClick {
                    id,
                    menu_item_id: menu_item_id.to_string(),
                    checked,
                })
                .is_ok()
        });
        drop(state);
        if checked.is_some() {
            self.changed.notify_one();
        }
        if sent {
            Ok(())
        } else {
            Err(MenuItemClickError::Unreachable)
        }
    }

    /// Sends `item.click` to the client that owns `key` (`"s3/counter"`).
    pub fn click(&self, key: &str, button: MouseButton) -> Result<(), ClickError> {
        let (session, id) = parse_key(key).ok_or(ClickError::NotFound)?;
        let state = self.lock();
        if !state
            .items
            .iter()
            .any(|i| i.session == session && i.id == id)
        {
            return Err(ClickError::NotFound);
        }
        let outbox = &state
            .sessions
            .get(&session)
            .ok_or(ClickError::NotFound)?
            .outbox;
        // try_send never waits: a stuck client just loses the click instead of freezing us.
        outbox
            .try_send(BarMessage::ItemClick {
                id: id.to_string(),
                button,
            })
            .map_err(|_| ClickError::Unreachable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(reg: &Registry, name: &str) -> (SessionId, mpsc::Receiver<BarMessage>) {
        let (tx, rx) = mpsc::channel(4);
        (reg.add_session(name.into(), 1, tx), rx)
    }

    fn labels(reg: &Registry) -> Vec<(String, Option<String>)> {
        reg.snapshot()
            .into_iter()
            .map(|v| (v.id, v.label))
            .collect()
    }

    #[test]
    fn add_update_remove() {
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, Some("0".into()), None)
            .unwrap();
        assert_eq!(labels(&reg), [("s1/a".into(), Some("0".into()))]);

        reg.update(
            s,
            "a",
            ItemPatch {
                label: Some("1".into()),
                tooltip: Some("tip".into()),
                ..Default::default()
            },
        )
        .unwrap();
        let view = &reg.snapshot()[0];
        assert_eq!(view.label.as_deref(), Some("1"));
        assert_eq!(view.tooltip.as_deref(), Some("tip"));
        assert_eq!(view.app_name, "App");

        // An empty string clears, a missing field keeps.
        reg.update(
            s,
            "a",
            ItemPatch {
                label: Some(String::new()),
                ..Default::default()
            },
        )
        .unwrap();
        let view = &reg.snapshot()[0];
        assert_eq!(view.label, None);
        assert_eq!(view.tooltip.as_deref(), Some("tip"));

        reg.remove(s, "a").unwrap();
        assert!(reg.snapshot().is_empty());
    }

    #[test]
    fn rejects_duplicates_unknown_ids_and_too_many_items() {
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        assert_eq!(
            reg.create(s, "a".into(), None, None, None),
            Err(RegistryError::DuplicateId)
        );
        assert_eq!(
            reg.update(s, "nope", ItemPatch::default()),
            Err(RegistryError::NotFound)
        );
        assert_eq!(reg.remove(s, "nope"), Err(RegistryError::NotFound));
        for n in 1..MAX_ITEMS_PER_CLIENT {
            reg.create(s, format!("i{n}"), None, None, None).unwrap();
        }
        assert_eq!(
            reg.create(s, "extra".into(), None, None, None),
            Err(RegistryError::LimitReached)
        );
    }

    #[test]
    fn same_id_in_two_sessions_does_not_collide() {
        let reg = Registry::new();
        let (a, _ra) = session(&reg, "A");
        let (b, _rb) = session(&reg, "B");
        reg.create(a, "main".into(), None, Some("a".into()), None)
            .unwrap();
        reg.create(b, "main".into(), None, Some("b".into()), None)
            .unwrap();
        reg.update(
            a,
            "main",
            ItemPatch {
                label: Some("a2".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            labels(&reg),
            [
                ("s1/main".into(), Some("a2".into())),
                ("s2/main".into(), Some("b".into()))
            ]
        );
    }

    #[test]
    fn removing_a_session_removes_its_items_only() {
        let reg = Registry::new();
        let (a, _ra) = session(&reg, "A");
        let (b, _rb) = session(&reg, "B");
        reg.create(a, "x".into(), None, None, None).unwrap();
        reg.create(b, "y".into(), None, None, None).unwrap();
        reg.remove_session(a);
        assert_eq!(reg.snapshot().len(), 1);
        assert_eq!(reg.snapshot()[0].id, "s2/y");
        assert_eq!(reg.session_count(), 1);
    }

    #[test]
    fn hidden_items_are_not_in_the_snapshot() {
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        reg.update(
            s,
            "a",
            ItemPatch {
                visible: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(reg.snapshot().is_empty());
        assert_eq!(reg.debug_items().len(), 1);
        reg.update(
            s,
            "a",
            ItemPatch {
                visible: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reg.snapshot().len(), 1);
    }

    #[test]
    fn click_reaches_the_owner_only() {
        let reg = Registry::new();
        let (a, mut ra) = session(&reg, "A");
        let (b, mut rb) = session(&reg, "B");
        reg.create(a, "x".into(), None, None, None).unwrap();
        reg.create(b, "x".into(), None, None, None).unwrap();
        reg.click("s2/x", MouseButton::Right).unwrap();
        assert_eq!(
            rb.try_recv().unwrap(),
            BarMessage::ItemClick {
                id: "x".into(),
                button: MouseButton::Right
            }
        );
        assert!(ra.try_recv().is_err());
    }

    #[test]
    fn click_on_missing_or_malformed_key_fails() {
        let reg = Registry::new();
        let (a, _ra) = session(&reg, "A");
        reg.create(a, "x".into(), None, None, None).unwrap();
        for key in ["s1/nope", "s9/x", "x", "", "sx/x", "/x"] {
            assert_eq!(
                reg.click(key, MouseButton::Left),
                Err(ClickError::NotFound),
                "{key}"
            );
        }
    }

    #[test]
    fn click_on_a_full_queue_does_not_block() {
        let reg = Registry::new();
        let (tx, _rx) = mpsc::channel(1);
        let s = reg.add_session("A".into(), 1, tx);
        reg.create(s, "x".into(), None, None, None).unwrap();
        reg.click("s1/x", MouseButton::Left).unwrap();
        assert_eq!(
            reg.click("s1/x", MouseButton::Left),
            Err(ClickError::Unreachable)
        );
    }

    #[tokio::test]
    async fn changes_wake_the_waiter_once_per_burst() {
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "A");
        reg.create(s, "x".into(), None, None, None).unwrap();
        reg.update(
            s,
            "x",
            ItemPatch {
                label: Some("1".into()),
                ..Default::default()
            },
        )
        .unwrap();
        // Two changes happened before anyone waited: one stored wake-up, not two.
        tokio::time::timeout(std::time::Duration::from_millis(50), reg.wait_changed())
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), reg.wait_changed())
                .await
                .is_err()
        );
        // Re-sending the same label is not a change.
        reg.update(
            s,
            "x",
            ItemPatch {
                label: Some("1".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), reg.wait_changed())
                .await
                .is_err()
        );
    }

    fn menu_of(ids: &[&str]) -> Vec<MenuTemplateItem> {
        ids.iter()
            .map(|id| MenuTemplateItem {
                id: Some((*id).into()),
                kind: Some(barista_protocol::MenuItemType::Checkbox),
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn menus_open_instead_of_item_click_except_middle() {
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        assert!(!reg.opens_menu("s1/a", MouseButton::Left));
        reg.set_menu(s, "a", menu_of(&["m"])).unwrap();
        assert!(reg.opens_menu("s1/a", MouseButton::Left));
        assert!(reg.opens_menu("s1/a", MouseButton::Right));
        assert!(!reg.opens_menu("s1/a", MouseButton::Middle));
        // Hidden items have no menu to open.
        reg.update(
            s,
            "a",
            ItemPatch {
                visible: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(reg.menu("s1/a").is_none());
    }

    #[test]
    fn menu_click_flips_state_bumps_rev_and_reaches_the_owner() {
        let reg = Registry::new();
        let (s, mut rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        reg.set_menu(s, "a", menu_of(&["m"])).unwrap();
        let rev = reg.menu("s1/a").unwrap().rev;
        reg.menu_click("s1/a", "m").unwrap();
        let after = reg.menu("s1/a").unwrap();
        assert!(after.entries[0].checked);
        assert!(after.rev > rev);
        assert_eq!(
            rx.try_recv().unwrap(),
            BarMessage::MenuClick {
                id: "a".into(),
                menu_item_id: "m".into(),
                checked: Some(true)
            }
        );
        assert_eq!(
            reg.menu_click("s1/a", "zzz"),
            Err(MenuItemClickError::Menu(MenuClickError::NotFound))
        );
        assert_eq!(
            reg.menu_click("s9/a", "m"),
            Err(MenuItemClickError::NotFound)
        );
    }

    #[test]
    fn same_menu_again_is_not_a_change() {
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        reg.set_menu(s, "a", menu_of(&["m"])).unwrap();
        let rev = reg.menu("s1/a").unwrap().rev;
        reg.set_menu(s, "a", menu_of(&["m"])).unwrap();
        assert_eq!(reg.menu("s1/a").unwrap().rev, rev);
        assert_eq!(reg.set_menu(s, "b", vec![]), Err(RegistryError::NotFound));
    }

    #[test]
    fn a_dead_client_menu_click_is_unreachable() {
        let reg = Registry::new();
        let (s, rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        reg.set_menu(s, "a", menu_of(&["m"])).unwrap();
        drop(rx);
        assert_eq!(
            reg.menu_click("s1/a", "m"),
            Err(MenuItemClickError::Unreachable)
        );
    }
}
