//! The item registry: every status item and every connected client session. The one place that
//! knows what is in the bar; the UI renders what `snapshot()` returns.
//!
//! Items are keyed by (session, id), so two apps can both call their item "main". The UI gets a
//! single string key, `"<session>/<id>"` (for example `"s3/counter"`). Item ids can't contain `/`
//! (see `ipc::validate`), so that key is unambiguous.

pub mod anchor;
pub mod menu;

use barista_protocol::{
    BarMessage, ErrorCode, ItemInfo, MenuTemplateItem, MouseButton, ScreenRect,
    MAX_ITEMS_PER_CLIENT,
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

/// What a click with a given button does to an item (the click rule, ADR 0014).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickTarget {
    /// Goes to the app as `item.click`.
    App,
    /// Opens the item's menu.
    Menu,
    /// Shows the item's popover.
    Popover,
}

/// What the coordinator needs to show a popover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopoverTarget {
    /// The client's own id for the item (what goes on the wire).
    pub id: String,
    /// Requested size, logical px.
    pub size: (u32, u32),
    /// The client's process, for `AllowSetForegroundWindow` (from the pipe, never from the client).
    pub pid: u32,
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
    /// Requested popover size (logical px). `None` = no popover.
    popover: Option<(u32, u32)>,
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
    /// Source of `menu_rev`s. Registry-wide, so a removed and re-created item never reuses one.
    last_rev: u64,
}

#[derive(Default)]
pub struct Registry {
    // A plain std Mutex is fine: it is only held for quick map edits and never across an
    // `.await`, so it can't block the async runtime for long.
    state: Mutex<State>,
    /// Pinged after every change. `notify_one` keeps one permit if nobody is waiting yet, so
    /// a burst of changes becomes a single wake-up for the event emitter.
    changed: Notify,
    /// Pinged when only a menu changed (new menu, checked flip). Items in the bar look the
    /// same, so this skips `items-changed` and only refreshes an open menu.
    menu_changed: Notify,
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
            popover: None,
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
                    has_popover: i.popover.is_some(),
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
        let menu = menu::arrange(menu);
        let mut state = self.lock();
        state.last_rev += 1;
        let rev = state.last_rev;
        let item = state
            .items
            .iter_mut()
            .find(|i| i.session == session && i.id == id)
            .ok_or(RegistryError::NotFound)?;
        if item.menu == menu {
            return Ok(());
        }
        item.menu = menu;
        item.menu_rev = rev;
        drop(state);
        self.menu_changed.notify_one();
        Ok(())
    }

    /// Resolves when a menu changed since the last wait (see `menu_changed`).
    pub async fn wait_menu_changed(&self) {
        self.menu_changed.notified().await;
    }

    /// Gives an item a popover size (already validated), or removes it with `None`.
    pub fn set_popover(
        &self,
        session: SessionId,
        id: &str,
        size: Option<(u32, u32)>,
    ) -> Result<(), RegistryError> {
        let mut state = self.lock();
        let item = state
            .items
            .iter_mut()
            .find(|i| i.session == session && i.id == id)
            .ok_or(RegistryError::NotFound)?;
        if item.popover == size {
            return Ok(());
        }
        item.popover = size;
        drop(state);
        // The coordinator listens to `changed`: a removed popover must close.
        self.changed.notify_one();
        Ok(())
    }

    /// The click rule: left = popover, then menu, then app; right = menu, then app; middle = app.
    /// Hidden or missing items go to `App` (and `click` reports them as missing).
    pub fn target(&self, key: &str, button: MouseButton) -> ClickTarget {
        let mut state = self.lock();
        let Some(item) = state.item_mut(key).filter(|i| i.visible) else {
            return ClickTarget::App;
        };
        let (has_popover, has_menu) = (item.popover.is_some(), !item.menu.is_empty());
        match button {
            MouseButton::Left if has_popover => ClickTarget::Popover,
            MouseButton::Left | MouseButton::Right if has_menu => ClickTarget::Menu,
            _ => ClickTarget::App,
        }
    }

    /// The popover of a visible item whose owner is still connected.
    pub fn popover_target(&self, key: &str) -> Option<PopoverTarget> {
        let mut state = self.lock();
        let item = state.item_mut(key).filter(|i| i.visible)?;
        let (session, id, size) = (item.session, item.id.clone(), item.popover?);
        let pid = state.sessions.get(&session)?.pid;
        Some(PopoverTarget { id, size, pid })
    }

    /// Whether `key` is a visible item with a popover (for `sync`: gone = close).
    pub fn has_popover(&self, key: &str) -> bool {
        self.popover_target(key).is_some()
    }

    /// Whether this session has an item with this id.
    pub fn has_item(&self, session: SessionId, id: &str) -> bool {
        self.lock()
            .items
            .iter()
            .any(|i| i.session == session && i.id == id)
    }

    /// Sends a message to the owner of `key`, built from the client's own item id. Never waits:
    /// a stuck client just misses it.
    pub fn send_with(
        &self,
        key: &str,
        build: impl FnOnce(String) -> BarMessage,
    ) -> Result<(), ClickError> {
        let (session, id) = parse_key(key).ok_or(ClickError::NotFound)?;
        let state = self.lock();
        let outbox = &state
            .sessions
            .get(&session)
            .ok_or(ClickError::NotFound)?
            .outbox;
        outbox
            .try_send(build(id.to_string()))
            .map_err(|_| ClickError::Unreachable)
    }

    /// The revision of a visible item's menu, without building it. `None` = nothing to show.
    pub fn menu_rev(&self, key: &str) -> Option<u64> {
        let mut state = self.lock();
        let item = state.item_mut(key)?;
        (!item.menu.is_empty() && item.visible).then_some(item.menu_rev)
    }

    /// The menu of a visible item, ready to draw. `None` if the item is gone, hidden or has no menu.
    pub fn menu(&self, key: &str) -> Option<MenuSnapshot> {
        let mut state = self.lock();
        let item = state.item_mut(key)?;
        if item.menu.is_empty() || !item.visible {
            return None;
        }
        let (session, rev) = (item.session, item.menu_rev);
        // Build the view while holding the lock: no clone of the template needed.
        let menu = std::mem::take(&mut item.menu);
        let entries = state
            .sessions
            .get(&session)
            .map(|s| menu::view(&menu, &s.app_name));
        if let Some(item) = state.item_mut(key) {
            item.menu = menu;
        }
        Some(MenuSnapshot {
            entries: entries?,
            rev,
        })
    }

    /// The user picked entry `menu_item_id` in the menu of `key`. Applies checkbox/radio state
    /// and sends `menu.click` (with the new `checked`) to the app. The state only changes if
    /// the app got the message, so the bar and the app never disagree.
    pub fn menu_click(&self, key: &str, menu_item_id: &str) -> Result<(), MenuItemClickError> {
        let mut state = self.lock();
        let item = state.item_mut(key).ok_or(MenuItemClickError::NotFound)?;
        if item.menu.is_empty() || !item.visible {
            return Err(MenuItemClickError::NotFound);
        }
        // Work on a copy; it replaces the real menu only once the message is on its way.
        let mut next = item.menu.clone();
        let checked = menu::click(&mut next, menu_item_id).map_err(MenuItemClickError::Menu)?;
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
        if !sent {
            return Err(MenuItemClickError::Unreachable);
        }
        if checked.is_some() {
            state.last_rev += 1;
            let rev = state.last_rev;
            if let Some(item) = state.item_mut(key) {
                item.menu = next;
                item.menu_rev = rev;
            }
            drop(state);
            self.menu_changed.notify_one();
        }
        Ok(())
    }

    /// Sends `item.click` to the client that owns `key` (`"s3/counter"`).
    #[cfg(test)]
    pub fn click(&self, key: &str, button: MouseButton) -> Result<(), ClickError> {
        self.click_with_anchor(key, button, None)
    }

    /// Same, with the item's rect on screen so the app can place its own window.
    pub fn click_with_anchor(
        &self,
        key: &str,
        button: MouseButton,
        anchor: Option<ScreenRect>,
    ) -> Result<(), ClickError> {
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
                anchor,
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
                button: MouseButton::Right,
                anchor: None
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
        use ClickTarget::{App, Menu};
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        assert_eq!(reg.target("s1/a", MouseButton::Left), App);
        reg.set_menu(s, "a", menu_of(&["m"])).unwrap();
        assert_eq!(reg.target("s1/a", MouseButton::Left), Menu);
        assert_eq!(reg.target("s1/a", MouseButton::Right), Menu);
        assert_eq!(reg.target("s1/a", MouseButton::Middle), App);
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
    fn the_click_rule_with_a_popover_a_menu_or_both() {
        use ClickTarget::{App, Menu, Popover};
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        reg.set_popover(s, "a", Some((320, 380))).unwrap();
        // Popover only: left opens it, right and middle go to the app.
        assert_eq!(reg.target("s1/a", MouseButton::Left), Popover);
        assert_eq!(reg.target("s1/a", MouseButton::Right), App);
        assert_eq!(reg.target("s1/a", MouseButton::Middle), App);
        // Both: left = popover, right = menu, middle = app.
        reg.set_menu(s, "a", menu_of(&["m"])).unwrap();
        assert_eq!(reg.target("s1/a", MouseButton::Left), Popover);
        assert_eq!(reg.target("s1/a", MouseButton::Right), Menu);
        assert_eq!(reg.target("s1/a", MouseButton::Middle), App);
        // Removing the popover gives the left click back to the menu.
        reg.set_popover(s, "a", None).unwrap();
        assert_eq!(reg.target("s1/a", MouseButton::Left), Menu);
        assert_eq!(reg.set_popover(s, "ghost", None), Err(RegistryError::NotFound));
    }

    #[test]
    fn popover_target_needs_a_visible_item_with_a_live_session() {
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        assert!(reg.popover_target("s1/a").is_none());
        reg.set_popover(s, "a", Some((320, 380))).unwrap();
        let t = reg.popover_target("s1/a").unwrap();
        assert_eq!((t.id.as_str(), t.size, t.pid), ("a", (320, 380), 1));
        reg.update(s, "a", ItemPatch { visible: Some(false), ..Default::default() }).unwrap();
        assert!(!reg.has_popover("s1/a"));
        reg.update(s, "a", ItemPatch { visible: Some(true), ..Default::default() }).unwrap();
        reg.remove_session(s);
        assert!(!reg.has_popover("s1/a"));
    }

    #[test]
    fn send_with_reaches_the_owner_and_never_blocks() {
        let reg = Registry::new();
        let (s, mut rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        reg.send_with("s1/a", |id| BarMessage::PopoverHide { id }).unwrap();
        assert_eq!(rx.try_recv().unwrap(), BarMessage::PopoverHide { id: "a".into() });
        assert_eq!(
            reg.send_with("s9/a", |id| BarMessage::PopoverHide { id }),
            Err(ClickError::NotFound)
        );
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

    #[test]
    fn an_undelivered_click_does_not_flip_the_checkbox() {
        let reg = Registry::new();
        let (tx, rx) = mpsc::channel(1);
        let s = reg.add_session("App".into(), 1, tx);
        reg.create(s, "a".into(), None, None, None).unwrap();
        reg.set_menu(s, "a", menu_of(&["m"])).unwrap();
        reg.menu_click("s1/a", "m").unwrap(); // fills the one-slot queue
        assert_eq!(
            reg.menu_click("s1/a", "m"),
            Err(MenuItemClickError::Unreachable)
        );
        assert!(
            reg.menu("s1/a").unwrap().entries[0].checked,
            "still the delivered state"
        );
        drop(rx);
    }

    #[test]
    fn hidden_items_menus_cannot_be_clicked() {
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        reg.set_menu(s, "a", menu_of(&["m"])).unwrap();
        reg.update(
            s,
            "a",
            ItemPatch {
                visible: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            reg.menu_click("s1/a", "m"),
            Err(MenuItemClickError::NotFound)
        );
    }

    #[test]
    fn a_recreated_item_never_reuses_a_menu_rev() {
        let reg = Registry::new();
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "a".into(), None, None, None).unwrap();
        reg.set_menu(s, "a", menu_of(&["m"])).unwrap();
        let first = reg.menu_rev("s1/a").unwrap();
        reg.remove(s, "a").unwrap();
        reg.create(s, "a".into(), None, None, None).unwrap();
        reg.set_menu(s, "a", menu_of(&["other"])).unwrap();
        assert_ne!(reg.menu_rev("s1/a").unwrap(), first);
    }
}
