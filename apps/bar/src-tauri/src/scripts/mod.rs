//! Script items (v0.9, ADRs 0034 and 0035): a runner built into the bar. Scripts in
//! `%APPDATA%\Barista\scripts` run on a timer and their output goes straight into the registry.
//!
//! ```text
//! folder_watch ─► 300 ms debounce ─► rescan ─► per script: trust? enabled? interval?
//!                                                   │
//!                                  one tokio task per script: verify hash ► run ► parse ► registry
//! ```
//! Each script has its own task and its own small state, so one that hangs or fails only ever
//! delays itself. `reconcile` is the one place that compares the folder with what is running.

pub mod actions;
pub mod menu;
pub mod name;
pub mod parse;
pub mod run;
pub mod scan;
pub mod trust;

use crate::controller;
use crate::registry::{Registry, ScriptItem, ScriptState};
use crate::settings::ScriptsPrefs;
use menu::{Action, Builtin};
use name::{Interpreter, ScriptName};
use run::{RunError, RunSpec};
use scan::{Found, FoundKind};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager as _};
use tauri_specta::Event;
use tokio::sync::Notify;
use trust::{sha256_hex, TrustStore};

/// Wait this long after the last file event, so an editor's save (several writes) is one rescan.
const DEBOUNCE: Duration = Duration::from_millis(300);
/// A locked file is tried again this often, this many times in a row.
const RETRY_AFTER: Duration = Duration::from_secs(2);
const MAX_RETRIES: u32 = 5;
/// The trust panel opens this long after a new script appears, so its item has a place in the bar.
const PANEL_DELAY: Duration = Duration::from_millis(600);

// ---------------------------------------------------------------- what the UI gets

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ScriptListState {
    /// Trusted and running on its timer (or about to).
    Running,
    /// Its last run failed.
    Error,
    /// Waiting for the user's yes.
    NeedsTrust,
    /// Switched off here, or scripts are off.
    Disabled,
    /// Not usable (bad file name, too big, same name as another file).
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScriptInfo {
    pub id: String,
    pub file_name: String,
    pub title: String,
    pub interpreter: String,
    /// How often it runs now (file name, else its setting, else the default).
    pub interval_secs: u32,
    /// The file name sets the interval, so the setting has no effect.
    pub interval_from_name: bool,
    /// This script's own switch (not counting the global one).
    pub enabled: bool,
    pub state: ScriptListState,
    /// The error, or why the file is not usable.
    pub message: Option<String>,
}

/// What the trust dialog shows. Internal: not part of the SDK or the protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TrustRequest {
    pub id: String,
    pub file_name: String,
    pub interpreter: String,
    pub size: u32,
    /// Hex SHA-256. Send it back with the answer: it only counts for this exact file.
    pub sha256: String,
    pub preview: Vec<String>,
}

/// The list of scripts changed. Carries the whole list.
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct ScriptsChanged(pub Vec<ScriptInfo>);

/// The scripts waiting for a yes changed. Carries all of them, oldest file name first; an empty
/// list means nothing is waiting any more (close the dialog).
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct ScriptTrustRequested(pub Vec<TrustRequest>);

// ---------------------------------------------------------------- the manager

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunKey {
    path: PathBuf,
    interval: u32,
    sha: String,
}

enum LiveState {
    /// Started, no result yet.
    Waiting,
    Ok,
    Error(String),
    NeedsTrust,
    Disabled,
}

struct Live {
    file_name: String,
    path: PathBuf,
    size: u64,
    sha256: String,
    preview: Vec<String>,
    name: ScriptName,
    interval_secs: u32,
    pref_enabled: bool,
    state: LiveState,
    task: Option<tauri::async_runtime::JoinHandle<()>>,
    key: Option<RunKey>,
    /// Tells a stale task's late result from the current one's.
    generation: u64,
    actions: HashMap<String, Action>,
    refresh: Arc<Notify>,
    running: Arc<AtomicBool>,
}

impl Live {
    fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            // Dropping the task's future drops the run's Job Object, which kills the process tree.
            task.abort();
        }
        self.key = None;
        self.running.store(false, Ordering::SeqCst);
    }
}

struct Inner {
    trust: TrustStore,
    live: HashMap<String, Live>,
    /// (file name, why) for files that are not usable.
    invalid: Vec<(String, String)>,
    generation: u64,
    last_infos: Vec<ScriptInfo>,
    last_pending: Vec<TrustRequest>,
}

struct Manager {
    app: AppHandle,
    registry: Arc<Registry>,
    dir: PathBuf,
    inner: Mutex<Inner>,
    /// Pinged by the folder watcher (and by tasks that spot an edit): rescan soon.
    rescan: Arc<Notify>,
    retries: AtomicU32,
}

static MANAGER: OnceLock<Arc<Manager>> = OnceLock::new();

fn prefs() -> ScriptsPrefs {
    controller::get().map(|c| c.settings().scripts).unwrap_or_default()
}

fn scripts_default() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("Barista").join("scripts")
}

/// `%APPDATA%\Barista\scripts`. Debug builds (and `BARISTA_DEBUG`) can point it elsewhere with
/// `BARISTA_SCRIPTS_DIR`, for tests and the bench; a release bar ignores it.
fn scripts_dir() -> PathBuf {
    let allow_override = cfg!(debug_assertions) || std::env::var_os("BARISTA_DEBUG").is_some();
    match std::env::var_os("BARISTA_SCRIPTS_DIR").filter(|_| allow_override) {
        Some(dir) => PathBuf::from(dir),
        None => scripts_default(),
    }
}

fn untrusted_item(title: &str) -> ScriptItem {
    ScriptItem {
        title: title.to_string(),
        label: None,
        color: None,
        state: ScriptState::Untrusted,
        tooltip: Some("Click to review this script before it runs".into()),
        menu: vec![],
    }
}

/// What a finished run puts in the bar: the item, what its menu lines do, and the error if any.
fn outcome(title: &str, result: Result<Vec<u8>, RunError>) -> (ScriptItem, HashMap<String, Action>, Option<String>) {
    let failed = |message: String, detail: String| {
        let built = menu::error_menu(&message, &detail);
        let item = ScriptItem {
            title: title.to_string(),
            label: None,
            color: None,
            state: ScriptState::Error,
            tooltip: Some(message.clone()),
            menu: built.entries,
        };
        (item, built.actions, Some(message))
    };
    match result {
        Err(e) => failed(e.message(), e.detail().to_string()),
        Ok(bytes) => match parse::parse(&bytes) {
            Err(e) => failed(e.message().to_string(), String::new()),
            Ok(out) => {
                let built = menu::from_output(&out.menu);
                let item = ScriptItem {
                    title: title.to_string(),
                    label: Some(out.label),
                    color: out.color,
                    state: ScriptState::Ok,
                    tooltip: None,
                    menu: built.entries,
                };
                (item, built.actions, None)
            }
        },
    }
}

impl Manager {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while the lock was held leaves the data usable, so ignore the poison flag.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Compares the folder with what is running and fixes the difference: new files start (or ask
    /// for trust), removed files stop, edited files ask again, a renamed interval restarts the
    /// timer. A rescan can't miss or double an event the way pairing watcher events could.
    async fn reconcile(self: &Arc<Self>, announce: bool) {
        let dir = self.dir.clone();
        let found = tokio::task::spawn_blocking(move || scan::scan(&dir)).await.unwrap_or_default();
        let prefs = prefs();
        let mut any_unreadable = false;
        let mut newly_pending: Vec<String> = Vec::new();
        {
            let mut inner = self.lock();
            inner.invalid.clear();
            let mut seen: HashSet<String> = HashSet::new();
            for f in found {
                match f.kind.clone() {
                    FoundKind::Invalid(why) => inner.invalid.push((f.file_name, why)),
                    FoundKind::Duplicate(_) => inner.invalid.push((f.file_name, "Another file has the same name".into())),
                    // Leave what is running alone: the file is only locked for a moment.
                    FoundKind::Unreadable(name) => {
                        any_unreadable = true;
                        seen.insert(name.id);
                    }
                    FoundKind::Script(name) => {
                        seen.insert(name.id.clone());
                        self.apply(&mut inner, f, name, &prefs, &mut newly_pending);
                    }
                }
            }
            let gone: Vec<String> = inner.live.keys().filter(|id| !seen.contains(*id)).cloned().collect();
            for id in gone {
                if let Some(mut live) = inner.live.remove(&id) {
                    live.stop();
                }
                self.registry.script_remove(&id);
            }
        }
        self.publish();

        if any_unreadable && self.retries.fetch_add(1, Ordering::SeqCst) < MAX_RETRIES {
            let wake = self.rescan.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(RETRY_AFTER).await;
                wake.notify_one();
            });
        } else if !any_unreadable {
            self.retries.store(0, Ordering::SeqCst);
        }
        if announce {
            if let Some(id) = newly_pending.first() {
                open_trust_panel_later(format!("script/{id}"));
            }
        }
    }

    /// One script found in the folder: decide what it should be doing and make it so.
    fn apply(&self, inner: &mut Inner, found: Found, name: ScriptName, prefs: &ScriptsPrefs, newly_pending: &mut Vec<String>) {
        let id = name.id.clone();
        let pref = prefs.per_script.get(&id);
        let pref_enabled = pref.map_or(true, |p| p.enabled);
        let enabled = prefs.enabled && pref_enabled;
        let interval = scan::effective_interval(name.interval_secs, pref.and_then(|p| p.interval_secs), prefs.default_interval_secs);
        let trusted = inner.trust.is_trusted(&id, &found.sha256);

        let live = inner.live.entry(id.clone()).or_insert_with(|| Live {
            file_name: String::new(),
            path: PathBuf::new(),
            size: 0,
            sha256: String::new(),
            preview: vec![],
            name: name.clone(),
            interval_secs: interval,
            pref_enabled,
            state: LiveState::Disabled,
            task: None,
            key: None,
            generation: 0,
            actions: HashMap::new(),
            refresh: Arc::new(Notify::new()),
            running: Arc::new(AtomicBool::new(false)),
        });
        let already_asked = matches!(live.state, LiveState::NeedsTrust) && live.sha256 == found.sha256;
        live.file_name = found.file_name;
        live.path = found.path.clone();
        live.size = found.size;
        live.sha256 = found.sha256.clone();
        live.preview = found.preview;
        live.name = name.clone();
        live.interval_secs = interval;
        live.pref_enabled = pref_enabled;

        if !enabled {
            live.stop();
            live.state = LiveState::Disabled;
            self.registry.script_remove(&id);
        } else if !trusted {
            // Nothing runs until the user says yes. An edited script lands here too.
            live.stop();
            live.state = LiveState::NeedsTrust;
            live.actions.clear();
            self.registry.script_set(&id, untrusted_item(&name.title));
            if !already_asked {
                newly_pending.push(id);
            }
        } else {
            let key = RunKey { path: found.path, interval, sha: found.sha256 };
            if live.key.as_ref() != Some(&key) || live.task.is_none() {
                live.stop();
                inner.generation += 1;
                live.generation = inner.generation;
                live.key = Some(key.clone());
                if matches!(live.state, LiveState::NeedsTrust | LiveState::Disabled) {
                    live.state = LiveState::Waiting;
                }
                // Keep the last output while the timer restarts; only a brand-new item gets a placeholder.
                if !self.registry.script_ids().contains(&id) {
                    self.registry.script_set(
                        &id,
                        ScriptItem {
                            title: name.title.clone(),
                            label: None,
                            color: None,
                            state: ScriptState::Ok,
                            tooltip: None,
                            menu: vec![],
                        },
                    );
                }
                if let Some(mgr) = MANAGER.get().cloned() {
                    live.task = Some(tauri::async_runtime::spawn(run_loop(
                        mgr,
                        id,
                        live.generation,
                        name.title,
                        key,
                        name.interpreter,
                        live.refresh.clone(),
                        live.running.clone(),
                    )));
                }
            }
        }
    }

    /// A run finished: show it. `true` = it worked (resets the failure back-off).
    fn finish(&self, id: &str, generation: u64, title: &str, result: Result<Vec<u8>, RunError>) -> bool {
        let (item, actions, error) = outcome(title, result);
        let ok = error.is_none();
        {
            let mut inner = self.lock();
            let Some(live) = inner.live.get_mut(id).filter(|l| l.generation == generation) else {
                return ok; // the script was removed or restarted meanwhile: drop the result
            };
            live.actions = actions;
            live.state = match error {
                None => LiveState::Ok,
                Some(message) => LiveState::Error(message),
            };
            self.registry.script_set(id, item);
        }
        self.publish();
        ok
    }

    /// Sends `scripts-changed` and `script-trust-requested` when their content changed.
    fn publish(&self) {
        let prefs = prefs();
        let (infos, pending, infos_changed, pending_changed) = {
            let mut inner = self.lock();
            let mut live: Vec<(&String, &Live)> = inner.live.iter().collect();
            live.sort_by_key(|(_, l)| l.file_name.to_lowercase());
            let mut infos: Vec<ScriptInfo> = live
                .iter()
                .map(|(id, l)| {
                    let (state, message) = match &l.state {
                        _ if !prefs.enabled || !l.pref_enabled => (ScriptListState::Disabled, None),
                        LiveState::NeedsTrust => (ScriptListState::NeedsTrust, None),
                        LiveState::Error(m) => (ScriptListState::Error, Some(m.clone())),
                        LiveState::Disabled => (ScriptListState::Disabled, None),
                        LiveState::Ok | LiveState::Waiting => (ScriptListState::Running, None),
                    };
                    ScriptInfo {
                        id: (*id).clone(),
                        file_name: l.file_name.clone(),
                        title: l.name.title.clone(),
                        interpreter: l.name.interpreter.label().to_string(),
                        interval_secs: l.interval_secs,
                        interval_from_name: l.name.interval_secs.is_some(),
                        enabled: l.pref_enabled,
                        state,
                        message,
                    }
                })
                .collect();
            let pending: Vec<TrustRequest> = live
                .iter()
                .filter(|(_, l)| matches!(l.state, LiveState::NeedsTrust) && prefs.enabled && l.pref_enabled)
                .map(|(id, l)| TrustRequest {
                    id: (*id).clone(),
                    file_name: l.file_name.clone(),
                    interpreter: l.name.interpreter.label().to_string(),
                    size: l.size.min(u32::MAX as u64) as u32,
                    sha256: l.sha256.clone(),
                    preview: l.preview.clone(),
                })
                .collect();
            for (file_name, why) in &inner.invalid {
                infos.push(ScriptInfo {
                    id: format!("invalid:{file_name}"),
                    file_name: file_name.clone(),
                    title: file_name.clone(),
                    interpreter: String::new(),
                    interval_secs: 0,
                    interval_from_name: false,
                    enabled: false,
                    state: ScriptListState::Invalid,
                    message: Some(why.clone()),
                });
            }
            let infos_changed = infos != inner.last_infos;
            let pending_changed = pending != inner.last_pending;
            inner.last_infos = infos.clone();
            inner.last_pending = pending.clone();
            (infos, pending, infos_changed, pending_changed)
        };
        if infos_changed {
            let _ = ScriptsChanged(infos).emit(&self.app);
        }
        if pending_changed {
            let _ = ScriptTrustRequested(pending).emit(&self.app);
        }
    }

    fn set_enabled(self: &Arc<Self>, id: &str, enabled: bool) {
        let id = id.to_string();
        crate::commands::settings::change(&self.app, |s| {
            s.scripts.per_script.entry(id).or_default().enabled = enabled;
        });
        self.reconcile_soon();
    }

    fn reconcile_soon(self: &Arc<Self>) {
        let mgr = self.clone();
        tauri::async_runtime::spawn(async move { mgr.reconcile(false).await });
    }

    fn refresh_now(&self, id: &str) {
        let inner = self.lock();
        if let Some(live) = inner.live.get(id).filter(|l| !l.running.load(Ordering::SeqCst)) {
            live.refresh.notify_one();
        }
    }
}

/// One script's life: check the file is still the one the user trusted, run it, show the result,
/// wait, repeat. A run that is still going can't overlap the next, because this loop is the only
/// thing that starts runs. Aborting the task kills the process tree (the run's Job Object).
#[allow(clippy::too_many_arguments)]
async fn run_loop(
    mgr: Arc<Manager>,
    id: String,
    generation: u64,
    title: String,
    key: RunKey,
    interpreter: Interpreter,
    refresh: Arc<Notify>,
    running: Arc<AtomicBool>,
) {
    let mut failures = 0u32;
    loop {
        // The hash is checked again before every run: an edit since the user said yes means no run.
        let (path, expected) = (key.path.clone(), key.sha.clone());
        let unchanged = tokio::task::spawn_blocking(move || std::fs::read(path).map(|b| sha256_hex(&b) == expected).unwrap_or(false))
            .await
            .unwrap_or(false);
        if !unchanged {
            mgr.rescan.notify_one();
            return;
        }
        running.store(true, Ordering::SeqCst);
        let result = run::run(RunSpec {
            path: key.path.clone(),
            interpreter,
            timeout: scan::timeout_for(key.interval),
            cwd: mgr.dir.clone(),
        })
        .await;
        running.store(false, Ordering::SeqCst);
        let ok = mgr.finish(&id, generation, &title, result);
        failures = if ok { 0 } else { failures.saturating_add(1) };
        tokio::select! {
            _ = tokio::time::sleep(scan::next_delay(key.interval, failures)) => {}
            // "Run again" or a click on the item: run now and forget the back-off.
            _ = refresh.notified() => failures = 0,
        }
    }
}

/// Opens the trust panel on `key`, if no other panel is up. Waits a moment: the item has to be
/// in the bar first, or the panel has nothing to hang from.
fn open_trust_panel_later(key: String) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(PANEL_DELAY).await;
        if let Some(panel) = crate::panel::get().filter(|p| p.current().is_none()) {
            panel.open("script-trust", &key, (380.0, 380.0), crate::panel::PanelChrome::Glass);
        }
    });
}

// ---------------------------------------------------------------- what the rest of the bar calls

/// Starts the runner: makes the folder, loads the trust list, watches the folder, runs what is
/// trusted. Call once from `setup`, after the settings are loaded.
pub fn init(app: &AppHandle, registry: Arc<Registry>) {
    let dir = scripts_dir();
    let _ = std::fs::create_dir_all(&dir);
    // The trust file sits with the settings. With a folder override it sits beside that folder.
    let trust_path = if dir != scripts_default() {
        dir.parent().map(|p| p.join("script-trust.json")).unwrap_or_else(|| dir.join("script-trust.json"))
    } else {
        app.path()
            .app_config_dir()
            .map(|d| d.join("script-trust.json"))
            .unwrap_or_else(|_| dir.join("script-trust.json"))
    };
    let manager = Arc::new(Manager {
        app: app.clone(),
        registry,
        dir: dir.clone(),
        inner: Mutex::new(Inner {
            trust: TrustStore::load(trust_path),
            live: HashMap::new(),
            invalid: vec![],
            generation: 0,
            last_infos: vec![],
            last_pending: vec![],
        }),
        rescan: Arc::new(Notify::new()),
        retries: AtomicU32::new(0),
    });
    if MANAGER.set(manager.clone()).is_err() {
        return;
    }
    // No polling: one thread sleeps until Windows reports a change in the folder.
    let wake = manager.rescan.clone();
    crate::platform::windows::folder_watch::start(dir, move || wake.notify_one());
    tauri::async_runtime::spawn(async move {
        manager.reconcile(false).await;
        loop {
            manager.rescan.notified().await;
            tokio::time::sleep(DEBOUNCE).await;
            manager.reconcile(true).await;
        }
    });
}

/// The bar is closing: stop every script. Windows would kill the jobs anyway when our handles
/// close, but this does it in order.
pub fn shutdown() {
    if let Some(manager) = MANAGER.get() {
        for live in manager.lock().live.values_mut() {
            live.stop();
        }
    }
}

/// Settings changed (scripts on or off, an interval, a script's switch): apply it now.
pub fn apply_settings() {
    if let Some(manager) = MANAGER.get() {
        manager.reconcile_soon();
    }
}

pub fn list() -> Vec<ScriptInfo> {
    MANAGER.get().map(|m| m.lock().last_infos.clone()).unwrap_or_default()
}

pub fn trust_requests() -> Vec<TrustRequest> {
    MANAGER.get().map(|m| m.lock().last_pending.clone()).unwrap_or_default()
}

/// The user answered a trust request. `allow` trusts exactly the file with this hash; the file is
/// read again first, so a script edited while the dialog was open is refused and asked again.
/// Not allowing switches the script off (turn it on in settings to be asked again).
pub fn answer_trust(id: &str, sha256: &str, allow: bool) -> Result<(), String> {
    let manager = MANAGER.get().ok_or("scripts are not running")?;
    if !allow {
        manager.set_enabled(id, false);
        return Ok(());
    }
    let path = {
        let inner = manager.lock();
        let live = inner.live.get(id).ok_or("That script is gone")?;
        if live.sha256 != sha256 {
            return Err("The script changed. Review it again.".into());
        }
        live.path.clone()
    };
    let now = std::fs::read(&path).map_err(|e| e.to_string())?;
    if sha256_hex(&now) != sha256 {
        manager.rescan.notify_one();
        return Err("The script changed. Review it again.".into());
    }
    if let Err(e) = manager.lock().trust.trust(id, sha256) {
        // Still trusted until the bar restarts; say so in the log, not to the user.
        eprintln!("barista: could not save script trust: {e}");
    }
    manager.reconcile_soon();
    Ok(())
}

/// Opens the scripts folder in Explorer.
pub fn open_folder() {
    if let Some(manager) = MANAGER.get() {
        crate::platform::windows::shell::open_uri(&manager.dir.to_string_lossy());
    }
}

/// A click on a script item that has no menu to open: review it if it waits for trust, else run now.
pub fn item_click(key: &str) {
    let (Some(manager), Some(id)) = (MANAGER.get(), key.strip_prefix("script/")) else { return };
    let waiting = manager.lock().live.get(id).is_some_and(|l| matches!(l.state, LiveState::NeedsTrust));
    if waiting {
        if let Some(panel) = crate::panel::get() {
            panel.open("script-trust", key, (380.0, 380.0), crate::panel::PanelChrome::Glass);
        }
    } else {
        manager.refresh_now(id);
    }
}

/// The user picked a line in a script's menu.
pub fn menu_click(key: &str, menu_id: &str) {
    let (Some(manager), Some(id)) = (MANAGER.get(), key.strip_prefix("script/")) else { return };
    let (action, path) = {
        let inner = manager.lock();
        let Some(live) = inner.live.get(id) else { return };
        (live.actions.get(menu_id).cloned(), live.path.clone())
    };
    let Some(action) = action else { return }; // plain text line: nothing to do
    if let Some(href) = &action.href {
        actions::open_href(href);
    }
    if let Some(shell) = &action.shell {
        actions::run_shell(shell, &manager.dir);
    }
    if action.refresh {
        manager.refresh_now(id);
    }
    match action.builtin {
        Some(Builtin::RunAgain) => manager.refresh_now(id),
        Some(Builtin::OpenScript) => actions::edit_script(&path),
        Some(Builtin::Disable) => manager.set_enabled(id, false),
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_good_run_becomes_an_ok_item_with_actions() {
        let (item, actions, error) = outcome("cpu", Ok(b"CPU 12%\n---\nOpen | href=https://example.com color=red".to_vec()));
        assert!(error.is_none());
        assert_eq!(item.state, ScriptState::Ok);
        assert_eq!(item.label.as_deref(), Some("CPU 12%"));
        assert_eq!(item.menu.len(), 1);
        assert!(actions.values().any(|a| a.href.is_some()));
    }

    #[test]
    fn a_failed_run_becomes_an_error_item_with_the_error_menu() {
        let err = RunError::Exit { code: Some(1), stderr: "boom".into() };
        let (item, actions, error) = outcome("cpu", Err(err));
        assert_eq!(item.state, ScriptState::Error);
        assert_eq!(error.as_deref(), Some("The script exited with code 1"));
        assert_eq!(item.menu[0].label, "Show error");
        assert!(actions.contains_key("run-again") && actions.contains_key("open-script") && actions.contains_key("disable"));
    }

    #[test]
    fn garbage_and_empty_output_are_errors() {
        for bytes in [vec![0u8, 1, 2], vec![], b"  \n".to_vec(), vec![b'x'; 70_000]] {
            let (item, _, error) = outcome("x", Ok(bytes));
            assert_eq!(item.state, ScriptState::Error);
            assert!(error.is_some());
        }
    }

    #[test]
    fn a_timeout_and_a_missing_interpreter_are_errors_not_crashes() {
        for e in [RunError::Timeout(Duration::from_secs(10)), RunError::NoInterpreter("Python not found".into()), RunError::TooLarge] {
            let (item, _, error) = outcome("x", Err(e));
            assert_eq!(item.state, ScriptState::Error);
            assert!(error.is_some());
        }
    }

    #[test]
    fn an_error_item_shows_the_file_title_as_its_label() {
        let (item, _, _) = outcome("system-monitor", Err(RunError::TooLarge));
        assert!(item.label.is_none());
        assert_eq!(item.title, "system-monitor");
    }
}
