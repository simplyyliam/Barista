//! The item registry: every status item and every connected client session. The one place that
//! knows what is in the bar; the UI renders what `snapshot()` returns.
//!
//! Items are keyed by (session, id), so two apps can both call their item "main". The UI gets a
//! single string key, `"<session>/<id>"` (for example `"s3/counter"`). Item ids can't contain `/`
//! (see `ipc::validate`), so that key is unambiguous.

pub mod anchor;
pub mod layout;
pub mod menu;

use barista_protocol::{
    BarMessage, ErrorCode, ItemInfo, ItemSource, MenuTemplateItem, MouseButton, ScreenRect,
    MAX_ITEMS_PER_CLIENT,
};
use crate::settings::{ItemLayout, MAX_LAYOUT_ENTRIES};
use layout::Placement;
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
    /// `app` for items clients made, `tray` for icons from the Windows tray (v0.6).
    pub source: ItemSource,
    /// Pinned items stay in the bar while it is collapsed (the user's choice, else the default:
    /// client items pinned, tray icons not). Since v0.7.
    pub pinned: bool,
    /// `bar` or `overflow` (the list behind the arrow). The list is in display order; draw `bar`
    /// ones in the bar and `overflow` ones in the overflow panel. Since v0.7.
    pub placement: Placement,
    /// A CSS color for the label (script items, v0.9).
    pub color: Option<String>,
    /// Set for script items: `ok`, `error` (draw the error mark) or `untrusted` (waiting for the user).
    pub script: Option<ScriptState>,
}

/// What a script item is doing, for the UI's badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ScriptState {
    Ok,
    Error,
    Untrusted,
}

/// A script item as the script manager hands it over. Its key in the UI is `script/<id>`.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptItem {
    /// The file name without interval and extension: the label while there is no output.
    pub title: String,
    pub label: Option<String>,
    pub color: Option<String>,
    pub state: ScriptState,
    pub tooltip: Option<String>,
    /// Ready to draw (ids already given). The manager keeps what each id does.
    pub menu: Vec<MenuEntry>,
}

struct StoredScript {
    item: ScriptItem,
    menu_rev: u64,
}

/// One title of the menu bar of the app in front (ADR 0036). Key `appmenu/<index>`.
struct StoredAppMenu {
    /// The items are read from the app each time it is opened (UI Automation), not kept.
    lazy: bool,
    entries: Vec<MenuEntry>,
    rev: u64,
}

/// Where to send a tray icon's clicks: the owner's window, the icon's id, its callback message and
/// the `NOTIFYICON_VERSION` it asked for (ADR 0027). Handles are the 32-bit numbers tray messages carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayOwner {
    pub hwnd: u32,
    pub id: u32,
    pub callback_message: Option<u32>,
    pub version: u32,
}

/// A Windows tray icon as the tray module hands it over. Its key in the UI is `tray/<key>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayItem {
    pub app_name: String,
    pub pid: u32,
    pub owner: TrayOwner,
    /// Survives a restart, for the saved layout (`tray:discord/1`).
    pub stable_id: String,
    /// A `data:` URL.
    pub icon: Option<String>,
    pub tooltip: Option<String>,
    /// Icons the app marked hidden are kept but not drawn.
    pub visible: bool,
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
    /// A visible Windows tray icon: the click goes to the app that owns it (ADR 0027).
    Tray,
    /// A script item without a menu: the script manager decides (trust prompt, or run now).
    Script,
    /// An application menu title whose items are read when it is opened (UI Automation apps).
    AppMenu,
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
    /// Tray icons (no session, no menu yet), keyed without the `tray/` prefix, in arrival order.
    tray: Vec<(String, TrayItem)>,
    /// Script items (v0.9), keyed by script id without the `script/` prefix, in arrival order.
    scripts: Vec<(String, StoredScript)>,
    /// The menu titles of the app in front, in order (ADR 0036). Not in the item list: the bar draws them on the left.
    app_menus: Vec<StoredAppMenu>,
    /// The user's order, pins and collapsed flag, saved in settings (ADR 0028).
    layout: ItemLayout,
    /// Room the UI has for items, and the gap between two (logical px). `None` = unknown.
    space: Option<(f64, f64)>,
    /// Last width each item had in the bar, by UI key. Kept after an item moves to overflow so it
    /// does not flip back and forth.
    widths: HashMap<String, f64>,
    /// Hide the icons Explorer itself puts in the tray (Safely Remove, Bluetooth, ...).
    hide_windows_icons: bool,
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

/// A width for an item the UI has not reported yet (logical px).
fn guess_width(view: &StatusItemView) -> f64 {
    let icon = if view.icon.is_some() || view.label.is_none() { 28.0 } else { 0.0 };
    let label = view.label.as_ref().map_or(0.0, |l| 14.0 + 7.0 * l.chars().count() as f64);
    icon + label
}

/// The layout id that survives a restart, for the item with this UI key.
fn stable_of(state: &State, key: &str) -> Option<String> {
    if let Some(tray_key) = key.strip_prefix("tray/") {
        return state.tray.iter().find(|(k, _)| k == tray_key).map(|(_, t)| t.stable_id.clone());
    }
    if let Some(id) = key.strip_prefix("script/") {
        return state.scripts.iter().any(|(k, _)| k == id).then(|| format!("script:{id}"));
    }
    let (session, id) = parse_key(key)?;
    state.items.iter().find(|i| i.session == session && i.id == id)?;
    Some(format!("app:{}/{}", state.sessions.get(&session)?.app_name, id))
}

/// Every visible item in display order with its pin and placement (ADR 0028).
fn arranged(state: &State) -> Vec<StatusItemView> {
    // Arrival order: client items, then tray icons.
    let mut views: Vec<Option<StatusItemView>> = Vec::new();
    let mut stables: Vec<String> = Vec::new();
    for i in state.items.iter().filter(|i| i.visible) {
        let Some(session) = state.sessions.get(&i.session) else { continue };
        stables.push(format!("app:{}/{}", session.app_name, i.id));
        views.push(Some(StatusItemView {
            id: format!("s{}/{}", i.session, i.id),
            app_name: session.app_name.clone(),
            icon: i.icon.clone(),
            label: i.label.clone(),
            tooltip: i.tooltip.clone(),
            source: ItemSource::App,
            pinned: false,
            placement: Placement::Bar,
            color: None,
            script: None,
        }));
    }
    for (id, s) in &state.scripts {
        stables.push(format!("script:{id}"));
        views.push(Some(StatusItemView {
            id: format!("script/{id}"),
            app_name: "Script".into(),
            icon: None,
            label: Some(s.item.label.clone().filter(|l| !l.is_empty()).unwrap_or_else(|| s.item.title.clone())),
            tooltip: s.item.tooltip.clone(),
            source: ItemSource::App,
            pinned: false,
            placement: Placement::Bar,
            color: s.item.color.clone(),
            script: Some(s.item.state),
        }));
    }
    for (key, t) in state.tray.iter().filter(|(_, t)| {
        t.visible && !(state.hide_windows_icons && t.app_name.eq_ignore_ascii_case("explorer"))
    }) {
        stables.push(t.stable_id.clone());
        views.push(Some(StatusItemView {
            id: format!("tray/{key}"),
            app_name: t.app_name.clone(),
            icon: t.icon.clone(),
            label: None,
            tooltip: t.tooltip.clone(),
            source: ItemSource::Tray,
            pinned: false,
            placement: Placement::Bar,
            color: None,
            script: None,
        }));
    }
    let entries: Vec<layout::Entry> = views
        .iter()
        .flatten()
        .zip(stables)
        .map(|(v, stable)| layout::Entry {
            width: state.widths.get(&v.id).copied().unwrap_or_else(|| guess_width(v)),
            source: v.source,
            stable,
        })
        .collect();
    layout::arrange(&entries, &state.layout, state.space)
        .into_iter()
        .filter_map(|p| {
            let mut view = views[p.index].take()?;
            view.pinned = p.pinned;
            view.placement = p.placement;
            Some(view)
        })
        .collect()
}

/// What the UI would redraw for: order, pins and placements (not the pictures).
fn shape(views: &[StatusItemView]) -> Vec<(String, bool, Placement)> {
    views.iter().map(|v| (v.id.clone(), v.pinned, v.placement)).collect()
}

#[derive(Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// A key that is not an item right now.
    NotFound,
    /// The same item twice, or too many.
    Invalid,
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

    /// The visible items as the UI draws them: in the saved order (items not in it come after, client
    /// items first, then tray icons, in arrival order), each with `pinned` and `placement`.
    pub fn snapshot(&self) -> Vec<StatusItemView> {
        arranged(&self.lock())
    }

    /// Ids (as the UI sees them) of the items drawn in the bar right now.
    pub fn ids_in_bar(&self) -> std::collections::HashSet<String> {
        self.snapshot()
            .into_iter()
            .filter(|v| v.placement == Placement::Bar)
            .map(|v| v.id)
            .collect()
    }

    /// Adds or updates a tray icon. Only a real change wakes the UI: some apps re-send the same
    /// tip every second.
    pub fn tray_set(&self, key: &str, item: TrayItem) {
        let mut state = self.lock();
        match state.tray.iter_mut().find(|(k, _)| k == key) {
            Some((_, existing)) => {
                if *existing == item {
                    return;
                }
                // The owner and pid are for clicks; the UI never shows them. Keep them current
                // without waking the UI when nothing visible changed.
                let silent = existing.app_name == item.app_name
                    && existing.stable_id == item.stable_id
                    && existing.icon == item.icon
                    && existing.tooltip == item.tooltip
                    && existing.visible == item.visible;
                *existing = item;
                if silent {
                    return;
                }
            }
            None => state.tray.push((key.to_string(), item)),
        }
        drop(state);
        self.changed.notify_one();
    }

    /// How many tray icons we know about (hidden ones too).
    pub fn tray_len(&self) -> usize {
        self.lock().tray.len()
    }

    /// Removes a tray icon (the app quit or took it away).
    pub fn tray_remove(&self, key: &str) {
        let mut state = self.lock();
        let before = state.tray.len();
        state.tray.retain(|(k, _)| k != key);
        let removed = state.tray.len() != before;
        drop(state);
        if removed {
            self.changed.notify_one();
        }
    }

    /// The "hide Windows' own icons" setting. Wakes the UI only when it changes.
    pub fn set_hide_windows_icons(&self, hide: bool) {
        let mut state = self.lock();
        if state.hide_windows_icons == hide {
            return;
        }
        state.hide_windows_icons = hide;
        drop(state);
        self.changed.notify_one();
    }

    /// Drops every tray icon (the tray was switched to Windows' own).
    pub fn tray_clear(&self) {
        let mut state = self.lock();
        let had = !state.tray.is_empty();
        state.tray.clear();
        drop(state);
        if had {
            self.changed.notify_one();
        }
    }

    /// Adds or updates a script item. A change to what the bar draws wakes the UI; a change to
    /// only the menu refreshes an open menu and nothing else, like an SDK menu change.
    pub fn script_set(&self, id: &str, item: ScriptItem) {
        let mut state = self.lock();
        state.last_rev += 1;
        let rev = state.last_rev;
        let (bar_changed, menu_changed) = match state.scripts.iter_mut().find(|(k, _)| k == id) {
            Some((_, stored)) => {
                let old = &stored.item;
                let bar = (&old.title, &old.label, &old.color, old.state, &old.tooltip)
                    != (&item.title, &item.label, &item.color, item.state, &item.tooltip);
                let menu = old.menu != item.menu;
                if menu {
                    stored.menu_rev = rev;
                }
                stored.item = item;
                (bar, menu)
            }
            None => {
                state.scripts.push((id.to_string(), StoredScript { item, menu_rev: rev }));
                (true, true)
            }
        };
        drop(state);
        if bar_changed {
            self.changed.notify_one();
        }
        if menu_changed {
            self.menu_changed.notify_one();
        }
    }

    /// Takes a script item out (file deleted, script disabled or scripts turned off).
    pub fn script_remove(&self, id: &str) {
        let mut state = self.lock();
        let before = state.scripts.len();
        state.scripts.retain(|(k, _)| k != id);
        let removed = state.scripts.len() != before;
        drop(state);
        if removed {
            self.changed.notify_one();
            self.menu_changed.notify_one();
        }
    }

    /// The ids of every script item right now.
    pub fn script_ids(&self) -> Vec<String> {
        self.lock().scripts.iter().map(|(k, _)| k.clone()).collect()
    }

    /// Replaces the menu titles of the app in front. Each is (read when opened, its items). An open app menu
    /// whose title is gone closes (the menu window syncs after every menu change).
    pub fn app_menus_replace(&self, menus: Vec<(bool, Vec<MenuEntry>)>) {
        let mut state = self.lock();
        let mut rev = state.last_rev;
        state.app_menus = menus
            .into_iter()
            .map(|(lazy, entries)| {
                rev += 1;
                StoredAppMenu { lazy, entries, rev }
            })
            .collect();
        state.last_rev = rev;
        drop(state);
        self.menu_changed.notify_one();
    }

    /// New items for one title (after the app was read again). A change of content moves its revision, so an
    /// open menu redraws.
    pub fn app_menu_entries(&self, index: usize, entries: Vec<MenuEntry>) {
        let mut state = self.lock();
        state.last_rev += 1;
        let rev = state.last_rev;
        let Some(menu) = state.app_menus.get_mut(index) else { return };
        if menu.entries == entries {
            return;
        }
        menu.entries = entries;
        menu.rev = rev;
        drop(state);
        self.menu_changed.notify_one();
    }

    /// How many app menu titles there are.
    #[cfg(test)]
    pub fn app_menu_count(&self) -> usize {
        self.lock().app_menus.len()
    }

    /// Loads the saved layout (once, at startup, before the first items arrive).
    pub fn set_layout(&self, layout: ItemLayout) {
        self.lock().layout = layout.cleaned();
        self.changed.notify_one();
    }

    /// The layout as it is now, to save in settings.
    pub fn layout(&self) -> ItemLayout {
        self.lock().layout.clone()
    }

    /// Changes the layout through `edit` and wakes the UI only if what it draws changed.
    fn edit_layout(
        &self,
        edit: impl FnOnce(&mut State) -> Result<(), LayoutError>,
    ) -> Result<(), LayoutError> {
        let mut state = self.lock();
        let before = shape(&arranged(&state));
        edit(&mut state)?;
        let changed = shape(&arranged(&state)) != before;
        drop(state);
        if changed {
            self.changed.notify_one();
        }
        Ok(())
    }

    /// Pin or unpin an item (by its UI key). Saved by the caller.
    pub fn set_pinned(&self, key: &str, pinned: bool) -> Result<(), LayoutError> {
        self.edit_layout(|state| {
            let stable = stable_of(state, key).ok_or(LayoutError::NotFound)?;
            // Only a choice that differs from the default is worth saving: it keeps the map small,
            // so the cap in `cleaned` does not drop a pin the user just made.
            let default = key.strip_prefix("tray/").is_none();
            if pinned == default {
                state.layout.pinned.remove(&stable);
            } else {
                state.layout.pinned.insert(stable, pinned);
            }
            state.layout = std::mem::take(&mut state.layout).cleaned();
            Ok(())
        })
    }

    /// The user arranged these items (UI keys, left to right). Only they move, within the slots they
    /// held, so items that are not running keep their place. Keys that are gone are skipped; a
    /// repeated key refuses the call.
    pub fn set_order(&self, keys: &[String]) -> Result<(), LayoutError> {
        if keys.len() > MAX_LAYOUT_ENTRIES {
            return Err(LayoutError::Invalid);
        }
        self.edit_layout(|state| {
            let mut seen = std::collections::HashSet::new();
            let mut stables: Vec<String> = Vec::new();
            for key in keys {
                if !seen.insert(key.as_str()) {
                    return Err(LayoutError::Invalid);
                }
                // An item can quit between the UI's list and the drop: skip it, keep the rest.
                let Some(stable) = stable_of(state, key) else { continue };
                // Two items can share a stable id (same program, same icon id): order them as one.
                if !stables.contains(&stable) {
                    stables.push(stable);
                }
            }
            state.layout.order = layout::merge_order(&state.layout.order, &stables);
            state.layout = std::mem::take(&mut state.layout).cleaned();
            Ok(())
        })
    }

    /// Collapsed = only pinned items stay in the bar.
    pub fn set_collapsed(&self, collapsed: bool) {
        let _ = self.edit_layout(|state| {
            state.layout.collapsed = collapsed;
            Ok(())
        });
    }

    pub fn collapsed(&self) -> bool {
        self.lock().layout.collapsed
    }

    /// The room the UI has for items (logical px) and the gap between two. `None` = no limit.
    /// Not saved: it depends on the screen.
    pub fn set_space(&self, space: Option<(f64, f64)>) {
        let space = space
            .filter(|(width, gap)| width.is_finite() && gap.is_finite())
            .map(|(width, gap)| (width.clamp(0.0, 20_000.0), gap.clamp(0.0, 100.0)));
        let _ = self.edit_layout(|state| {
            state.space = space;
            Ok(())
        });
    }

    /// Widths the UI measured (`setItemBounds`), keyed by item id. A change under half a pixel is noise,
    /// and an item in overflow is not measured in the bar, so its remembered width stays.
    pub fn note_widths(&self, widths: impl IntoIterator<Item = (String, f64)>) {
        let widths: Vec<(String, f64)> = widths
            .into_iter()
            .filter(|(_, w)| (1.0..=2000.0).contains(w))
            .collect();
        // Cheap check first: the UI reports on every layout pass, mostly with nothing new.
        let state = self.lock();
        let changed = widths.iter().any(|(id, w)| {
            state.widths.get(id).map_or(true, |old| (old - w).abs() > 0.5)
        });
        if !changed {
            return;
        }
        drop(state);
        let _ = self.edit_layout(|state| {
            let in_bar: std::collections::HashSet<String> = arranged(state)
                .into_iter()
                .filter(|v| v.placement == Placement::Bar)
                .map(|v| v.id)
                .collect();
            for (id, width) in widths {
                if !in_bar.contains(&id) {
                    continue;
                }
                if state.widths.len() >= 512 && !state.widths.contains_key(&id) {
                    state.widths.clear();
                }
                state.widths.insert(id, width);
            }
            Ok(())
        });
    }

    /// Where a visible tray icon's clicks go, with its owner's pid (for `AllowSetForegroundWindow`).
    /// `key` is the full UI key (`tray/<key>`). `None` when the icon is gone, hidden, or has no
    /// callback message (nothing to send to).
    pub fn tray_owner(&self, key: &str) -> Option<(TrayOwner, u32)> {
        let tray_key = key.strip_prefix("tray/")?;
        let state = self.lock();
        let (_, item) = state.tray.iter().find(|(k, _)| k == tray_key)?;
        (item.visible && item.owner.callback_message.is_some()).then_some((item.owner, item.pid))
    }

    /// Every item including hidden ones, with its owner's pid. For `debug.listItems`.
    pub fn debug_items(&self) -> Vec<ItemInfo> {
        let state = self.lock();
        let tray = state.tray.iter().map(|(key, t)| ItemInfo {
            id: format!("tray/{key}"),
            app_name: t.app_name.clone(),
            pid: t.pid,
            has_icon: t.icon.is_some(),
            label: None,
            tooltip: t.tooltip.clone(),
            visible: t.visible,
            has_menu: false,
            has_popover: false,
            source: ItemSource::Tray,
        });
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
                    source: ItemSource::App,
                })
            })
            .chain(tray)
            .chain(state.scripts.iter().map(|(id, s)| ItemInfo {
                id: format!("script/{id}"),
                app_name: "Script".into(),
                pid: 0,
                has_icon: false,
                label: s.item.label.clone().or_else(|| Some(s.item.title.clone())),
                tooltip: s.item.tooltip.clone(),
                visible: true,
                has_menu: !s.item.menu.is_empty(),
                has_popover: false,
                source: ItemSource::App,
            }))
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
        if let Some(tray_key) = key.strip_prefix("tray/") {
            let visible = state.tray.iter().any(|(k, t)| k == tray_key && t.visible);
            return if visible { ClickTarget::Tray } else { ClickTarget::App };
        }
        if let Some(index) = key.strip_prefix("appmenu/").and_then(|n| n.parse::<usize>().ok()) {
            return match state.app_menus.get(index) {
                Some(m) if m.lazy => ClickTarget::AppMenu,
                Some(m) if !m.entries.is_empty() => ClickTarget::Menu,
                _ => ClickTarget::App,
            };
        }
        if let Some(id) = key.strip_prefix("script/") {
            return match state.scripts.iter().find(|(k, _)| k == id) {
                // A menu opens on any button; without one the manager decides.
                Some((_, s)) if s.item.state != ScriptState::Untrusted && !s.item.menu.is_empty() => ClickTarget::Menu,
                Some(_) => ClickTarget::Script,
                None => ClickTarget::App,
            };
        }
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
        if let Some(index) = key.strip_prefix("appmenu/").and_then(|n| n.parse::<usize>().ok()) {
            let m = state.app_menus.get(index)?;
            return (!m.entries.is_empty()).then_some(m.rev);
        }
        if let Some(id) = key.strip_prefix("script/") {
            let (_, s) = state.scripts.iter().find(|(k, _)| k == id)?;
            return (!s.item.menu.is_empty() && s.item.state != ScriptState::Untrusted).then_some(s.menu_rev);
        }
        let item = state.item_mut(key)?;
        (!item.menu.is_empty() && item.visible).then_some(item.menu_rev)
    }

    /// The menu of a visible item, ready to draw. `None` if the item is gone, hidden or has no menu.
    pub fn menu(&self, key: &str) -> Option<MenuSnapshot> {
        let mut state = self.lock();
        if let Some(index) = key.strip_prefix("appmenu/").and_then(|n| n.parse::<usize>().ok()) {
            let m = state.app_menus.get(index)?;
            if m.entries.is_empty() {
                return None;
            }
            return Some(MenuSnapshot { entries: m.entries.clone(), rev: m.rev });
        }
        if let Some(id) = key.strip_prefix("script/") {
            let (_, s) = state.scripts.iter().find(|(k, _)| k == id)?;
            if s.item.menu.is_empty() || s.item.state == ScriptState::Untrusted {
                return None;
            }
            return Some(MenuSnapshot { entries: s.item.menu.clone(), rev: s.menu_rev });
        }
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

    fn tray(tip: &str, visible: bool) -> TrayItem {
        TrayItem {
            app_name: "Discord".into(),
            pid: 42,
            owner: TrayOwner { hwnd: 0x1234, id: 1, callback_message: Some(0x8001), version: 0 },
            stable_id: format!("tray:{}/1", tip.to_lowercase()),
            icon: Some("data:image/png;base64,AAAA".into()),
            tooltip: Some(tip.into()),
            visible,
        }
    }

    async fn woke(reg: &Registry) -> bool {
        tokio::time::timeout(std::time::Duration::from_millis(50), reg.wait_changed())
            .await
            .is_ok()
    }

    #[tokio::test]
    async fn tray_icons_come_after_app_items_with_their_source() {
        let reg = Registry::new();
        reg.tray_set("a1-0", tray("Discord", true));
        let (s, _rx) = session(&reg, "App");
        reg.create(s, "x".into(), None, None, None).unwrap();
        let view = reg.snapshot();
        assert_eq!(view.len(), 2);
        assert_eq!((view[0].id.as_str(), view[0].source), ("s1/x", ItemSource::App));
        assert_eq!((view[1].id.as_str(), view[1].source), ("tray/a1-0", ItemSource::Tray));
        assert_eq!(view[1].tooltip.as_deref(), Some("Discord"));
        assert_eq!(view[1].app_name, "Discord");
        let debug = reg.debug_items();
        assert_eq!(debug.iter().filter(|i| i.source == ItemSource::Tray).count(), 1);
        assert!(woke(&reg).await);
    }

    #[tokio::test]
    async fn tray_icons_update_and_leave_live() {
        let reg = Registry::new();
        reg.tray_set("k", tray("one", true));
        assert!(woke(&reg).await);
        reg.tray_set("k", tray("two", true));
        assert!(woke(&reg).await);
        assert_eq!(reg.snapshot()[0].tooltip.as_deref(), Some("two"));
        // The same values again are not a change (a monitor app re-sends its tip every second).
        reg.tray_set("k", tray("two", true));
        assert!(!woke(&reg).await);
        reg.tray_remove("k");
        assert!(woke(&reg).await);
        assert!(reg.snapshot().is_empty());
        // Removing what is not there is not a change either.
        reg.tray_remove("k");
        assert!(!woke(&reg).await);
    }

    #[tokio::test]
    async fn an_owner_change_updates_clicks_without_waking_the_ui() {
        let reg = Registry::new();
        reg.tray_set("k", tray("x", true));
        assert!(woke(&reg).await);
        let mut moved = tray("x", true);
        moved.owner.callback_message = Some(0x400);
        moved.pid = 99;
        reg.tray_set("k", moved);
        assert!(!woke(&reg).await);
        // The click still goes to the new callback.
        assert_eq!(reg.tray_owner("tray/k").map(|(o, pid)| (o.callback_message, pid)), Some((Some(0x400), 99)));
    }

    #[test]
    fn hidden_tray_icons_are_kept_but_not_drawn() {
        let reg = Registry::new();
        reg.tray_set("k", tray("x", false));
        assert!(reg.snapshot().is_empty());
        assert_eq!(reg.debug_items().len(), 1);
        reg.tray_set("k", tray("x", true));
        assert_eq!(reg.snapshot().len(), 1);
    }

    #[test]
    fn a_tray_click_targets_the_owner_and_nothing_else() {
        let reg = Registry::new();
        reg.tray_set("k", tray("x", true));
        for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
            assert_eq!(reg.target("tray/k", button), ClickTarget::Tray);
        }
        let (owner, pid) = reg.tray_owner("tray/k").unwrap();
        assert_eq!((owner.hwnd, owner.id, pid), (0x1234, 1, 42));
        // Hidden, missing, or without a callback message: nowhere to send it.
        reg.tray_set("h", tray("x", false));
        assert_eq!(reg.target("tray/h", MouseButton::Left), ClickTarget::App);
        assert!(reg.tray_owner("tray/h").is_none());
        assert!(reg.tray_owner("tray/nope").is_none());
        let mut silent = tray("x", true);
        silent.owner.callback_message = None;
        reg.tray_set("s", silent);
        assert!(reg.tray_owner("tray/s").is_none());
        // No client session or menu behind a tray key.
        assert_eq!(reg.click("tray/k", MouseButton::Left), Err(ClickError::NotFound));
        assert!(reg.menu("tray/k").is_none());
        assert!(!reg.has_popover("tray/k"));
        assert!(reg.popover_target("tray/k").is_none());
        assert_eq!(reg.menu_click("tray/k", "a"), Err(MenuItemClickError::NotFound));
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

    // ---- layout (ADR 0028) ----

    fn order_and_placement(reg: &Registry) -> Vec<(String, bool, Placement)> {
        reg.snapshot().into_iter().map(|v| (v.id, v.pinned, v.placement)).collect()
    }

    /// One client item and three tray icons.
    fn crowded() -> (Registry, mpsc::Receiver<BarMessage>) {
        let reg = Registry::new();
        let (s, rx) = session(&reg, "Timer");
        reg.create(s, "main".into(), None, Some("25:00".into()), None).unwrap();
        for tip in ["a", "b", "c"] {
            reg.tray_set(tip, tray(tip, true));
        }
        (reg, rx)
    }

    #[test]
    fn a_fresh_bar_is_collapsed_with_client_items_pinned() {
        let (reg, _rx) = crowded();
        assert_eq!(
            order_and_placement(&reg),
            [
                ("s1/main".into(), true, Placement::Bar),
                ("tray/a".into(), false, Placement::Overflow),
                ("tray/b".into(), false, Placement::Overflow),
                ("tray/c".into(), false, Placement::Overflow),
            ]
        );
    }

    #[test]
    fn pinning_and_expanding_bring_tray_icons_into_the_bar() {
        let (reg, _rx) = crowded();
        reg.set_pinned("tray/b", true).unwrap();
        let view = order_and_placement(&reg);
        assert_eq!(view[2], ("tray/b".into(), true, Placement::Bar));
        assert_eq!(view[1].2, Placement::Overflow);
        reg.set_collapsed(false);
        assert!(reg.snapshot().iter().all(|v| v.placement == Placement::Bar));
        assert!(!reg.collapsed());
        // A client item can be unpinned too: collapsed again, it goes to overflow.
        reg.set_pinned("s1/main", false).unwrap();
        reg.set_collapsed(true);
        assert_eq!(reg.snapshot()[0].placement, Placement::Overflow);
    }

    #[test]
    fn pins_use_stable_ids_so_a_restart_keeps_them() {
        let (reg, _rx) = crowded();
        reg.set_pinned("tray/b", true).unwrap();
        let saved = reg.layout();
        assert_eq!(saved.pinned.get("tray:b/1"), Some(&true));
        // A new run: different session number and window handle, same stable ids.
        let again = Registry::new();
        again.set_layout(saved);
        let (s, _rx2) = session(&again, "Timer");
        again.create(s, "main".into(), None, Some("25:00".into()), None).unwrap();
        again.tray_set("zz-9", { let mut t = tray("b", true); t.owner.hwnd = 0x7777; t });
        let view = order_and_placement(&again);
        assert_eq!(view[1], ("tray/zz-9".into(), true, Placement::Bar));
    }

    #[test]
    fn set_order_moves_only_the_given_items() {
        let (reg, _rx) = crowded();
        reg.set_order(&["tray/c".into(), "tray/a".into(), "s1/main".into()]).unwrap();
        let ids: Vec<_> = order_and_placement(&reg).into_iter().map(|v| v.0).collect();
        assert_eq!(ids, ["tray/c", "tray/a", "s1/main", "tray/b"].map(String::from));
        // Unknown or repeated keys are refused and change nothing.
        // A key that is gone is skipped (an app quit mid-drag); the rest still applies.
        assert_eq!(reg.set_order(&["tray/nope".into(), "tray/b".into()]), Ok(()));
        assert_eq!(reg.set_order(&["tray/a".into(), "tray/a".into()]), Err(LayoutError::Invalid));
        let still: Vec<_> = order_and_placement(&reg).into_iter().map(|v| v.0).collect();
        assert_eq!(still, ids);
        assert_eq!(reg.set_pinned("tray/nope", true), Err(LayoutError::NotFound));
    }

    #[tokio::test]
    async fn layout_changes_wake_the_ui_only_when_something_moves() {
        let (reg, _rx) = crowded();
        woke(&reg).await; // drain
        reg.set_collapsed(true); // already collapsed
        assert!(!woke(&reg).await);
        reg.set_collapsed(false);
        assert!(woke(&reg).await);
        reg.set_pinned("tray/a", false).unwrap(); // already unpinned by default, but now saved: nothing moves
        assert!(!woke(&reg).await);
        reg.set_pinned("tray/a", true).unwrap();
        assert!(woke(&reg).await);
    }

    #[tokio::test]
    async fn too_little_room_sends_unpinned_items_to_overflow() {
        let (reg, _rx) = crowded();
        reg.set_collapsed(false);
        woke(&reg).await;
        // 28 each for the three tray icons, a label item of 14 + 7 * 5 = 49: 49 + 3 * 28 + 3 * 2 = 139.
        reg.set_space(Some((139.0, 2.0)));
        assert!(!woke(&reg).await);
        assert!(reg.snapshot().iter().all(|v| v.placement == Placement::Bar));
        reg.set_space(Some((100.0, 2.0)));
        assert!(woke(&reg).await);
        let overflow: Vec<_> = reg.snapshot().into_iter().filter(|v| v.placement == Placement::Overflow).map(|v| v.id).collect();
        assert_eq!(overflow, ["tray/a", "tray/b"].map(String::from));
        // A measured width replaces the guess: the label item is really 20 wide, so one more fits.
        reg.note_widths([("s1/main".to_string(), 20.0)]);
        let in_bar = reg.snapshot().iter().filter(|v| v.placement == Placement::Bar).count();
        assert_eq!(in_bar, 3);
        // Nonsense input is ignored; no limit brings everything back.
        reg.set_space(Some((f64::NAN, 2.0)));
        reg.note_widths([("tray/a".to_string(), f64::INFINITY), ("tray/a".to_string(), -4.0)]);
        assert!(reg.snapshot().iter().all(|v| v.placement == Placement::Bar));
    }

    fn script_item(label: &str, state: ScriptState, menu: Vec<MenuEntry>) -> ScriptItem {
        ScriptItem { title: "cpu".into(), label: Some(label.into()), color: None, state, tooltip: None, menu }
    }
    fn entry(id: &str) -> MenuEntry {
        MenuEntry {
            id: Some(id.into()),
            kind: barista_protocol::MenuItemType::Normal,
            label: id.into(),
            checked: false,
            enabled: true,
            accelerator: None,
            role: None,
            icon: None,
            info: false,
            color: None,
            submenu: vec![],
        }
    }

    #[tokio::test]
    async fn a_script_item_shows_in_the_bar_and_goes_away() {
        let reg = Registry::new();
        reg.script_set("cpu.ps1", script_item("CPU 12%", ScriptState::Ok, vec![]));
        let items = reg.snapshot();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "script/cpu.ps1");
        assert_eq!(items[0].label.as_deref(), Some("CPU 12%"));
        assert_eq!(items[0].script, Some(ScriptState::Ok));
        reg.script_remove("cpu.ps1");
        assert!(reg.snapshot().is_empty());
    }

    #[tokio::test]
    async fn an_empty_label_falls_back_to_the_title() {
        let reg = Registry::new();
        reg.script_set("cpu.ps1", script_item("", ScriptState::Ok, vec![]));
        assert_eq!(reg.snapshot()[0].label.as_deref(), Some("cpu"));
    }

    #[tokio::test]
    async fn script_items_are_pinned_by_default_and_have_a_stable_id() {
        let reg = Registry::new();
        reg.script_set("cpu.ps1", script_item("x", ScriptState::Ok, vec![]));
        assert!(reg.snapshot()[0].pinned);
        assert_eq!(stable_of(&reg.lock(), "script/cpu.ps1").as_deref(), Some("script:cpu.ps1"));
        assert_eq!(stable_of(&reg.lock(), "script/nope"), None);
    }

    #[tokio::test]
    async fn clicks_go_to_the_menu_or_the_manager_never_to_an_app() {
        let reg = Registry::new();
        reg.script_set("a.ps1", script_item("x", ScriptState::Ok, vec![entry("m0")]));
        reg.script_set("b.ps1", script_item("x", ScriptState::Ok, vec![]));
        reg.script_set("c.ps1", script_item("x", ScriptState::Untrusted, vec![entry("m0")]));
        assert_eq!(reg.target("script/a.ps1", MouseButton::Left), ClickTarget::Menu);
        assert_eq!(reg.target("script/a.ps1", MouseButton::Right), ClickTarget::Menu);
        assert_eq!(reg.target("script/b.ps1", MouseButton::Left), ClickTarget::Script);
        // An untrusted script has no menu to show, even if one was stored.
        assert_eq!(reg.target("script/c.ps1", MouseButton::Left), ClickTarget::Script);
        assert!(reg.menu("script/c.ps1").is_none());
        assert_eq!(reg.target("script/gone", MouseButton::Left), ClickTarget::App);
    }

    #[tokio::test]
    async fn a_script_menu_is_served_with_a_revision_that_moves_only_when_it_changes() {
        let reg = Registry::new();
        reg.script_set("a.ps1", script_item("x", ScriptState::Ok, vec![entry("m0")]));
        let first = reg.menu("script/a.ps1").unwrap();
        assert_eq!(first.entries.len(), 1);
        reg.script_set("a.ps1", script_item("y", ScriptState::Ok, vec![entry("m0")]));
        assert_eq!(reg.menu_rev("script/a.ps1"), Some(first.rev), "same menu, same revision");
        reg.script_set("a.ps1", script_item("y", ScriptState::Ok, vec![entry("m0"), entry("m1")]));
        assert!(reg.menu_rev("script/a.ps1").unwrap() > first.rev);
    }

    #[tokio::test]
    async fn only_a_menu_change_skips_the_items_event() {
        let reg = Registry::new();
        reg.script_set("a.ps1", script_item("x", ScriptState::Ok, vec![entry("m0")]));
        // Drain the wake-ups of the first set.
        reg.wait_changed().await;
        reg.wait_menu_changed().await;
        reg.script_set("a.ps1", script_item("x", ScriptState::Ok, vec![entry("m0"), entry("m1")]));
        reg.wait_menu_changed().await;
        let waited = tokio::time::timeout(std::time::Duration::from_millis(50), reg.wait_changed()).await;
        assert!(waited.is_err(), "a menu-only change must not wake the bar");
    }

    #[tokio::test]
    async fn script_items_appear_in_the_debug_list_and_do_not_touch_app_sessions() {
        let reg = Registry::new();
        reg.script_set("a.ps1", script_item("x", ScriptState::Error, vec![entry("m0")]));
        let debug = reg.debug_items();
        assert_eq!(debug.len(), 1);
        assert!(debug[0].has_menu);
        assert_eq!(reg.session_count(), 0);
        assert_eq!(reg.script_ids(), vec!["a.ps1".to_string()]);
    }

    #[tokio::test]
    async fn app_menu_titles_get_keys_and_lazy_ones_are_read_on_click() {
        let reg = Registry::new();
        reg.app_menus_replace(vec![(false, vec![entry("a0")]), (true, vec![]), (false, vec![])]);
        assert_eq!(reg.app_menu_count(), 3);
        // A classic menu was read already: it opens at once. A UI Automation title is read when clicked.
        assert_eq!(reg.target("appmenu/0", MouseButton::Left), ClickTarget::Menu);
        assert_eq!(reg.target("appmenu/1", MouseButton::Left), ClickTarget::AppMenu);
        assert_eq!(reg.target("appmenu/2", MouseButton::Left), ClickTarget::App);
        assert_eq!(reg.target("appmenu/9", MouseButton::Left), ClickTarget::App);
        assert_eq!(reg.menu("appmenu/0").unwrap().entries.len(), 1);
        assert!(reg.menu("appmenu/2").is_none());
        // They are not bar items: the right side never draws them.
        assert!(reg.snapshot().is_empty());
    }

    #[tokio::test]
    async fn app_menu_entries_move_the_revision_only_when_they_change() {
        let reg = Registry::new();
        reg.app_menus_replace(vec![(true, vec![])]);
        reg.app_menu_entries(0, vec![entry("a0")]);
        let first = reg.menu_rev("appmenu/0").unwrap();
        reg.app_menu_entries(0, vec![entry("a0")]);
        assert_eq!(reg.menu_rev("appmenu/0"), Some(first));
        reg.app_menu_entries(0, vec![entry("a0"), entry("a1")]);
        assert!(reg.menu_rev("appmenu/0").unwrap() > first);
        // A title that is gone (the app changed) can't be filled in.
        reg.app_menus_replace(vec![]);
        reg.app_menu_entries(0, vec![entry("a0")]);
        assert_eq!(reg.app_menu_count(), 0);
        assert!(reg.menu("appmenu/0").is_none());
    }
}
