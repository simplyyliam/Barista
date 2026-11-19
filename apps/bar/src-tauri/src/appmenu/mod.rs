//! Application menus (ADR 0036): the menus of the app in front (File, Edit, View ...) in the bar.
//!
//! ```text
//! foreground window changes ─► 120 ms debounce ─► classic menu (GetMenu)?  ─► titles + all items
//!                                                  else UI Automation bar?  ─► titles (items read on open)
//!                                                  else                     ─► just the app name
//! ```
//! The menu text comes from another program, so it is capped and cleaned here (`entries_from`) before
//! the registry or the UI see it, and a click only ever posts a command id or invokes a UIA element in
//! the window the menu was read from.

use crate::controller;
use crate::platform::windows::app_info;
use crate::platform::windows::foreground;
use crate::platform::windows::uia_menu;
use crate::platform::windows::win32_menu::{self, Kind, RawItem};
use crate::registry::menu::MenuEntry;
use crate::registry::Registry;
use barista_protocol::MenuItemType;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;
use tauri::AppHandle;
use tauri_specta::Event;
use tokio::sync::Notify;

/// Wait this long after the last foreground change (Alt+Tab passes through several windows).
const DEBOUNCE: Duration = Duration::from_millis(120);
const MAX_TITLES: usize = 24;
const MAX_ENTRIES: usize = 300;
const MAX_DEPTH: usize = 3;
const MAX_LABEL_CHARS: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppMenuTitle {
    /// The key to click and to anchor the menu on (`appmenu/<n>`).
    pub key: String,
    pub label: String,
}

/// What the bar draws on the left: the app's name and its menu titles.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppMenus {
    /// Empty = nothing in front that we know (the desktop): the bar shows "Barista".
    pub app_name: String,
    pub titles: Vec<AppMenuTitle>,
}

/// The app in front changed, or its menu titles did. Carries everything.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct AppMenusChanged(pub AppMenus);

/// What clicking an entry does.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    /// A classic menu command: `WM_COMMAND`.
    Command(u32),
    /// A UI Automation item: the choice made at each level, and the names seen (checked again).
    Uia { path: Vec<usize>, names: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provider {
    None,
    Win32,
    Uia,
}

struct State {
    /// The window the menus were read from, and the process that owned it then. A click is only sent if
    /// the window still belongs to that process (a window handle can be reused).
    hwnd: isize,
    pid: u32,
    provider: Provider,
    titles: Vec<String>,
    /// Per title: entry id to what it does.
    actions: Vec<HashMap<String, Action>>,
    shown: AppMenus,
}

struct Service {
    app: AppHandle,
    registry: Arc<Registry>,
    state: Mutex<State>,
    wake: Notify,
}

static SERVICE: OnceLock<Arc<Service>> = OnceLock::new();
static LATEST: AtomicIsize = AtomicIsize::new(0);

// ---------------------------------------------------------------- cleaning what another app sent

fn clean_label(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).take(MAX_LABEL_CHARS).collect::<String>().trim().to_string()
}

/// Where the entries came from, to build what a click does.
#[derive(Clone)]
enum Source {
    Win32,
    Uia { title_index: usize, title: String },
}

/// Raw items into entries the bar can draw, plus what each does. Capped (count, depth, label length),
/// control characters stripped, no separator at an edge or twice in a row. Entry ids are `a0`, `a1` ...
fn entries_from(
    items: &[RawItem],
    source: &Source,
    path: &[(usize, String)],
    next_id: &mut usize,
    actions: &mut HashMap<String, Action>,
    depth: usize,
) -> Vec<MenuEntry> {
    let mut out: Vec<MenuEntry> = Vec::new();
    for item in items {
        if *next_id >= MAX_ENTRIES {
            break;
        }
        if item.kind == Kind::Separator {
            if out.last().is_some_and(|e| e.kind != MenuItemType::Separator) {
                out.push(entry(None, MenuItemType::Separator, String::new(), false, true, None, vec![]));
            }
            continue;
        }
        let label = clean_label(&item.label);
        if label.is_empty() {
            continue;
        }
        let id = format!("a{}", *next_id);
        *next_id += 1;
        let accelerator = item.accelerator.as_deref().map(clean_label).filter(|a| !a.is_empty());
        match item.kind {
            Kind::Submenu => {
                let mut here = path.to_vec();
                here.push((item.command as usize, label.clone()));
                let children = if depth < MAX_DEPTH { entries_from(&item.children, source, &here, next_id, actions, depth + 1) } else { vec![] };
                let kind = if children.is_empty() { MenuItemType::Normal } else { MenuItemType::Submenu };
                // A submenu with nothing readable is drawn disabled, not as a dead arrow.
                let enabled = item.enabled && !children.is_empty();
                out.push(entry(Some(id), kind, label, false, enabled, accelerator, children));
            }
            _ => {
                let action = match source {
                    Source::Win32 => Action::Command(item.command),
                    Source::Uia { title_index, title } => {
                        let mut p = vec![*title_index];
                        let mut names = vec![title.clone()];
                        for (index, name) in path {
                            p.push(*index);
                            names.push(name.clone());
                        }
                        p.push(item.command as usize);
                        names.push(label.clone());
                        Action::Uia { path: p, names }
                    }
                };
                actions.insert(id.clone(), action);
                let kind = if item.checked { MenuItemType::Checkbox } else { MenuItemType::Normal };
                out.push(entry(Some(id), kind, label, item.checked, item.enabled, accelerator, vec![]));
            }
        }
    }
    while out.last().is_some_and(|e| e.kind == MenuItemType::Separator) {
        out.pop();
    }
    out
}

fn entry(
    id: Option<String>,
    kind: MenuItemType,
    label: String,
    checked: bool,
    enabled: bool,
    accelerator: Option<String>,
    submenu: Vec<MenuEntry>,
) -> MenuEntry {
    MenuEntry { id, kind, label, checked, enabled, accelerator, role: None, icon: None, info: false, color: None, submenu }
}

// ---------------------------------------------------------------- which windows count

/// Windows we never show menus for: the desktop and the taskbar. (Barista's own windows are skipped
/// by process id: the bar's menu windows take focus when they open, and the menus must stay.)
const SHELL_CLASSES: [&str; 6] =
    ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd", "NotifyIconOverflowWindow", "TopLevelWindowForOverflowXamlIsland"];

enum Target {
    /// Not an app window to follow (Barista itself, or a window that is gone): leave things as they are.
    Ignore,
    /// The desktop: show no app.
    Desktop,
    App(isize, u32),
}

fn classify(hwnd: isize) -> Target {
    let root = app_info::root_window(hwnd);
    if root == 0 || !win32_menu::is_window(root) {
        return Target::Ignore;
    }
    let pid = app_info::pid_of(root);
    if pid == 0 || pid == std::process::id() {
        return Target::Ignore;
    }
    if SHELL_CLASSES.contains(&app_info::class_name(root).as_str()) {
        return Target::Desktop;
    }
    Target::App(root, pid)
}

/// The window's system menu (Restore, Move, Size, Minimize, Close): its commands are all `SC_*`
/// (0xF000 and up). Some apps (browsers) put it in their menu bar slot; it is not the app's menu.
fn is_system_menu(top: &RawItem) -> bool {
    let commands: Vec<u32> = top.children.iter().filter(|c| c.kind == Kind::Normal).map(|c| c.command).collect();
    !commands.is_empty() && commands.iter().all(|c| (0xF000..=0xFFFF).contains(c))
}

/// The submenu titles of a menu bar that are the app's own, in order.
fn app_titles(bar: &[RawItem]) -> Vec<&RawItem> {
    bar.iter().filter(|i| i.kind == Kind::Submenu && !is_system_menu(i)).take(MAX_TITLES).collect()
}

fn enabled_in_settings() -> bool {
    controller::get()
        .map(|c| c.settings().menu_bar.items.iter().find(|i| i.id == "app-menus").map_or(true, |i| i.visible))
        .unwrap_or(true)
}

// ---------------------------------------------------------------- the service

impl Service {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Reads the menus of the window that just came to the front. Runs on a blocking thread: reading a
    /// menu can wait on the other app (with short timeouts).
    fn refresh(&self, hwnd: isize) {
        if !enabled_in_settings() {
            self.show(0, Provider::None, String::new(), vec![], vec![], vec![]);
            return;
        }
        let (root, pid) = match classify(hwnd) {
            Target::Ignore => return,
            Target::Desktop => {
                self.show(0, Provider::None, String::new(), vec![], vec![], vec![]);
                return;
            }
            Target::App(root, pid) => (root, pid),
        };
        let name = app_info::display_name(pid).unwrap_or_default();
        if let Some(bar) = win32_menu::read_menu_bar(root) {
            let tops = app_titles(&bar);
            if !tops.is_empty() {
                let mut titles = Vec::new();
                let mut entries = Vec::new();
                let mut actions = Vec::new();
                for top in tops {
                    let mut map = HashMap::new();
                    let mut next_id = 0;
                    let items = entries_from(&top.children, &Source::Win32, &[], &mut next_id, &mut map, 0);
                    titles.push(clean_label(&top.label));
                    entries.push(items);
                    actions.push(map);
                }
                self.show(root, Provider::Win32, name, titles, entries, actions);
                return;
            }
        }
        if let Some(titles) = uia_menu::titles(root) {
            let titles: Vec<String> = titles.iter().map(|t| clean_label(t)).filter(|t| !t.is_empty()).take(MAX_TITLES).collect();
            if !titles.is_empty() {
                let n = titles.len();
                self.show(root, Provider::Uia, name, titles, vec![vec![]; n], vec![HashMap::new(); n]);
                return;
            }
        }
        self.show(root, Provider::None, name, vec![], vec![], vec![]);
    }

    /// Stores what was read, gives the registry the titles and tells the UI if anything changed.
    fn show(&self, hwnd: isize, provider: Provider, name: String, titles: Vec<String>, entries: Vec<Vec<MenuEntry>>, actions: Vec<HashMap<String, Action>>) {
        let lazy = provider == Provider::Uia;
        let shown = AppMenus {
            app_name: name,
            titles: titles.iter().enumerate().map(|(i, t)| AppMenuTitle { key: format!("appmenu/{i}"), label: t.clone() }).collect(),
        };
        let changed = {
            let mut state = self.lock();
            state.hwnd = hwnd;
            state.pid = if hwnd == 0 { 0 } else { app_info::pid_of(hwnd) };
            state.provider = provider;
            state.titles = titles;
            state.actions = actions;
            let changed = state.shown != shown;
            state.shown = shown.clone();
            changed
        };
        self.registry.app_menus_replace(entries.into_iter().map(|e| (lazy, e)).collect());
        if changed {
            let _ = AppMenusChanged(shown).emit(&self.app);
        }
    }
}

/// Starts following the app in front. Call once from `setup` (main thread: the hook needs its message loop).
pub fn init(app: &AppHandle, registry: Arc<Registry>) {
    let service = Arc::new(Service {
        app: app.clone(),
        registry,
        state: Mutex::new(State { hwnd: 0, pid: 0, provider: Provider::None, titles: vec![], actions: vec![], shown: AppMenus::default() }),
        wake: Notify::new(),
    });
    if SERVICE.set(service.clone()).is_err() {
        return;
    }
    foreground::watch_window(|hwnd| {
        LATEST.store(hwnd, Ordering::SeqCst);
        if let Some(service) = SERVICE.get() {
            service.wake.notify_one();
        }
    });
    LATEST.store(foreground::current_window(), Ordering::SeqCst);
    tauri::async_runtime::spawn(async move {
        loop {
            // The first pass (at startup) happens without an event.
            let hwnd = LATEST.load(Ordering::SeqCst);
            if hwnd != 0 {
                let svc = service.clone();
                let _ = tokio::task::spawn_blocking(move || svc.refresh(hwnd)).await;
            }
            service.wake.notified().await;
            tokio::time::sleep(DEBOUNCE).await;
        }
    });
}

/// The settings said app menus were switched on or off (the bar item's visibility): read again now.
pub fn settings_changed() {
    if let Some(service) = SERVICE.get() {
        service.wake.notify_one();
    }
}

/// The titles now (for `getAppMenus`, once when the bar starts).
pub fn get() -> AppMenus {
    SERVICE.get().map(|s| s.lock().shown.clone()).unwrap_or_default()
}

/// A click on a UI Automation title: read its menu from the app (it flashes the app's own popup for a
/// moment), give it to the registry, then open it. A click on the title that is already open closes it.
pub fn open_lazy(key: &str) {
    let Some(service) = SERVICE.get().cloned() else { return };
    let Some(index) = key.strip_prefix("appmenu/").and_then(|n| n.parse::<usize>().ok()) else { return };
    let key = key.to_string();
    if crate::menu_window::get().and_then(|m| m.current()).is_some_and(|open| open.item_id == key) {
        if let Some(menu) = crate::menu_window::get() {
            menu.item_click(&key);
        }
        return;
    }
    tauri::async_runtime::spawn(async move {
        let (hwnd, title) = {
            let state = service.lock();
            match state.titles.get(index) {
                Some(t) => (state.hwnd, t.clone()),
                None => return,
            }
        };
        let read = tokio::task::spawn_blocking(move || uia_menu::read_title(hwnd, index)).await.ok().flatten();
        let Some(items) = read else { return };
        let mut actions = HashMap::new();
        let mut next_id = 0;
        let entries = entries_from(&items, &Source::Uia { title_index: index, title }, &[], &mut next_id, &mut actions, 0);
        {
            let mut state = service.lock();
            // The user moved on to another app while we were reading: drop it.
            if state.hwnd != hwnd || index >= state.actions.len() {
                return;
            }
            state.actions[index] = actions;
        }
        if entries.is_empty() {
            return;
        }
        service.registry.app_menu_entries(index, entries);
        if let Some(menu) = crate::menu_window::get() {
            menu.item_click(&key);
        }
    });
}

/// A classic menu was opened: read it again in the background so enabled and checked states are
/// current (the open menu redraws when the entries change).
pub fn refresh_open() {
    let Some(service) = SERVICE.get().cloned() else { return };
    let hwnd = service.lock().hwnd;
    if hwnd == 0 || service.lock().provider != Provider::Win32 {
        return;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let Some(bar) = win32_menu::read_menu_bar(hwnd) else { return };
        let tops = app_titles(&bar);
        let mut state = service.lock();
        // Only if it is still the same app with the same titles.
        if state.hwnd != hwnd || tops.len() != state.titles.len() {
            return;
        }
        let mut updates = Vec::new();
        for (index, top) in tops.iter().enumerate() {
            let mut map = HashMap::new();
            let mut next_id = 0;
            let entries = entries_from(&top.children, &Source::Win32, &[], &mut next_id, &mut map, 0);
            state.actions[index] = map;
            updates.push((index, entries));
        }
        drop(state);
        for (index, entries) in updates {
            service.registry.app_menu_entries(index, entries);
        }
    });
}

/// The user picked a line in an app menu.
pub fn menu_click(key: &str, menu_id: &str) {
    let Some(service) = SERVICE.get().cloned() else { return };
    let Some(index) = key.strip_prefix("appmenu/").and_then(|n| n.parse::<usize>().ok()) else { return };
    let (hwnd, pid, action) = {
        let state = service.lock();
        (state.hwnd, state.pid, state.actions.get(index).and_then(|a| a.get(menu_id)).cloned())
    };
    let Some(action) = action else { return };
    // The window the menu was read from must still be there, and still belong to the same program.
    if hwnd == 0 || !win32_menu::is_window(hwnd) || app_info::pid_of(hwnd) != pid {
        return;
    }
    match action {
        Action::Command(command) => {
            // The click took the focus into the bar's menu window: give it back first, so the command
            // lands in the app's focused control (Paste into the right box).
            foreground::focus(hwnd);
            win32_menu::post_command(hwnd, command);
        }
        Action::Uia { path, names } => {
            foreground::focus(hwnd);
            tauri::async_runtime::spawn_blocking(move || {
                uia_menu::invoke(hwnd, path, names);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(label: &str, command: u32) -> RawItem {
        RawItem { kind: Kind::Normal, label: label.into(), accelerator: None, command, checked: false, enabled: true, children: vec![] }
    }
    fn sep() -> RawItem {
        RawItem { kind: Kind::Separator, label: String::new(), accelerator: None, command: 0, checked: false, enabled: true, children: vec![] }
    }
    fn sub(label: &str, children: Vec<RawItem>) -> RawItem {
        RawItem { kind: Kind::Submenu, label: label.into(), accelerator: None, command: 0, checked: false, enabled: true, children }
    }
    fn build(items: &[RawItem]) -> (Vec<MenuEntry>, HashMap<String, Action>) {
        let (mut next, mut actions) = (0, HashMap::new());
        let entries = entries_from(items, &Source::Win32, &[], &mut next, &mut actions, 0);
        (entries, actions)
    }

    #[test]
    fn commands_become_actions_by_entry_id() {
        let (entries, actions) = build(&[raw("New", 101), raw("Open", 102)]);
        assert_eq!(entries.len(), 2);
        assert_eq!(actions[entries[0].id.as_ref().unwrap()], Action::Command(101));
        assert_eq!(actions[entries[1].id.as_ref().unwrap()], Action::Command(102));
    }

    #[test]
    fn separators_never_sit_at_an_edge_or_twice() {
        let (entries, _) = build(&[sep(), raw("A", 1), sep(), sep(), raw("B", 2), sep()]);
        let kinds: Vec<_> = entries.iter().map(|e| e.kind).collect();
        assert_eq!(kinds, vec![MenuItemType::Normal, MenuItemType::Separator, MenuItemType::Normal]);
    }

    #[test]
    fn checked_items_draw_a_check_and_disabled_stay_disabled() {
        let mut checked = raw("Word wrap", 1);
        checked.checked = true;
        let mut off = raw("Exit", 2);
        off.enabled = false;
        let (entries, _) = build(&[checked, off]);
        assert_eq!((entries[0].kind, entries[0].checked), (MenuItemType::Checkbox, true));
        assert!(!entries[1].enabled);
    }

    #[test]
    fn accelerators_are_kept_and_cleaned() {
        let mut item = raw("Save", 1);
        item.accelerator = Some("Ctrl+S\u{7}".into());
        assert_eq!(build(&[item]).0[0].accelerator.as_deref(), Some("Ctrl+S"));
    }

    #[test]
    fn labels_are_cleaned_and_capped_and_empty_ones_dropped() {
        let long = "x".repeat(500);
        let (entries, _) = build(&[raw("Hello\u{1b}[31m world", 1), raw(&long, 2), raw("   ", 3), raw("", 4)]);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].label, "Hello[31m world");
        assert_eq!(entries[1].label.chars().count(), MAX_LABEL_CHARS);
    }

    #[test]
    fn submenus_nest_and_an_empty_one_is_disabled() {
        let (entries, actions) = build(&[sub("Zoom", vec![raw("In", 301), raw("Out", 302)]), sub("Empty", vec![])]);
        assert_eq!(entries[0].kind, MenuItemType::Submenu);
        assert_eq!(entries[0].submenu.len(), 2);
        assert_eq!(actions.len(), 2);
        assert!(!entries[1].enabled);
    }

    #[test]
    fn depth_is_capped() {
        let mut deepest = vec![raw("leaf", 1)];
        for level in 0..6 {
            deepest = vec![sub(&format!("level {level}"), deepest)];
        }
        let (entries, _) = build(&deepest);
        let mut depth = 0;
        let mut at = &entries;
        while let Some(first) = at.first() {
            depth += 1;
            at = &first.submenu;
        }
        assert!(depth <= MAX_DEPTH + 2, "depth {depth}");
    }

    #[test]
    fn the_entry_count_is_capped() {
        let many: Vec<RawItem> = (0..1000).map(|i| raw(&format!("Item {i}"), i)).collect();
        assert_eq!(build(&many).0.len(), MAX_ENTRIES);
    }

    #[test]
    fn uia_actions_carry_the_whole_path_and_the_names_seen() {
        let items = [sub("Zoom", vec![RawItem { command: 4, ..raw("In", 0) }])];
        let items = [RawItem { command: 2, ..items[0].clone() }];
        let (mut next, mut actions) = (0, HashMap::new());
        let source = Source::Uia { title_index: 1, title: "View".into() };
        let entries = entries_from(&items, &source, &[], &mut next, &mut actions, 0);
        let leaf = entries[0].submenu[0].id.as_ref().unwrap();
        assert_eq!(
            actions[leaf],
            Action::Uia { path: vec![1, 2, 4], names: vec!["View".into(), "Zoom".into(), "In".into()] }
        );
    }

    #[test]
    fn the_windows_system_menu_is_not_an_app_menu() {
        let system = sub("System", vec![raw("Restore", 0xF120), raw("Move", 0xF010), sep(), raw("Close", 0xF060)]);
        let file = sub("File", vec![raw("New", 101), raw("Close", 0xF060)]);
        let bar = [system, file];
        let titles = app_titles(&bar);
        assert_eq!(titles.len(), 1);
        assert_eq!(titles[0].label, "File");
    }

    #[test]
    fn menu_text_stays_text() {
        // Nothing in an entry is ever interpreted: a label that looks like markup or a command is just a label.
        let (entries, actions) = build(&[raw("<img src=x onerror=alert(1)>", 7), raw("calc.exe & del *", 8)]);
        assert_eq!(entries[0].label, "<img src=x onerror=alert(1)>");
        assert!(actions.values().all(|a| matches!(a, Action::Command(_))));
    }
}
