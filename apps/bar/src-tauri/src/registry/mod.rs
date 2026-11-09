//! The item registry: every status item and every connected client session. The one place that
//! knows what is in the bar; the UI renders what `snapshot()` returns.
//!
//! Items are keyed by (session, id), so two apps can both call their item "main". The UI gets a
//! single string key, `"<session>/<id>"` (for example `"s3/counter"`). Item ids can't contain `/`
//! (see `ipc::validate`), so that key is unambiguous.

use barista_protocol::{BarMessage, ErrorCode, ItemInfo, MouseButton, MAX_ITEMS_PER_CLIENT};
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
                    icon: i.icon.clone(),
                    label: i.label.clone(),
                    tooltip: i.tooltip.clone(),
                    visible: i.visible,
                })
            })
            .collect()
    }

    /// Sends `item.click` to the client that owns `key` (`"s3/counter"`).
    pub fn click(&self, key: &str, button: MouseButton) -> Result<(), ClickError> {
        let (session_text, id) = key.split_once('/').ok_or(ClickError::NotFound)?;
        let session = parse_session(session_text).ok_or(ClickError::NotFound)?;
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
}
