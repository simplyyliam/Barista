//! Auto-update (v0.10, ADR 0038). The installed bar asks the public releases repo for `latest.json`,
//! checks the signature with the public key in tauri.conf.json, and installs when the user says so.
//!
//! Until `pnpm updater:keygen` has put a public key in tauri.conf.json the plugin is not started
//! (it would stop the whole app from launching), and every command answers "not set up".

use crate::controller;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
use tauri_specta::Event;

/// First check this long after start-up, so it never competes with the bar appearing.
const FIRST_CHECK: Duration = Duration::from_secs(30);
/// One sleeping timer, not a polling loop: the bar stays at 0% CPU.
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    /// The release notes from the changelog, if the release has any.
    pub notes: Option<String>,
}

/// A newer version was found (by the timer or by "Check for updates").
#[derive(Debug, Clone, Serialize, Deserialize, Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAvailable(pub UpdateInfo);

/// The update found last, kept so `install_update` installs exactly what the user was shown.
#[derive(Default)]
struct Pending(Mutex<Option<Update>>);

const NOT_SET_UP: &str = "Updates are not set up in this build.";

/// True from the moment "Update now" is accepted. A check meanwhile must not swap the update that is
/// being downloaded, and a second click must not start a second download.
static INSTALLING: AtomicBool = AtomicBool::new(false);

/// True when tauri.conf.json carries the public key (the one-time `pnpm updater:keygen` step).
fn has_public_key(app: &AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|c| c.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.trim().is_empty())
}

pub fn init(app: &AppHandle) {
    app.manage(Pending::default());
    if !has_public_key(app) {
        log::warn!("updater: no public key in tauri.conf.json, updates are off (run pnpm updater:keygen)");
        return;
    }
    if let Err(error) = app.plugin(tauri_plugin_updater::Builder::new().build()) {
        log::error!("updater: could not start: {error}");
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            // Read every round, so switching the setting off takes effect without a restart.
            let on = controller::get().map_or(true, |c| c.settings().behavior.auto_update);
            if on {
                if let Err(error) = check(&app).await {
                    log::info!("updater: check failed: {error}");
                }
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    if !has_public_key(app) {
        return Err(NOT_SET_UP.into());
    }
    if INSTALLING.load(Ordering::SeqCst) {
        return Err("An update is being installed.".into());
    }
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let Some(update) = update else {
        *app.state::<Pending>().0.lock().unwrap_or_else(|e| e.into_inner()) = None;
        return Ok(None);
    };
    let info = UpdateInfo {
        version: update.version.clone(),
        notes: update.body.clone(),
    };
    // The same version again (the 6 hour timer): nothing changed, so keep what is held and say nothing.
    let state = app.state::<Pending>();
    let mut pending = state.0.lock().unwrap_or_else(|e| e.into_inner());
    if pending.as_ref().is_some_and(|p| p.version == update.version) {
        return Ok(Some(info));
    }
    *pending = Some(update);
    drop(pending);
    let _ = UpdateAvailable(info.clone()).emit(app);
    Ok(Some(info))
}

/// Asks the releases repo whether a newer version exists. `None` = you are up to date.
#[tauri::command]
#[specta::specta]
pub async fn check_for_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    check(&app).await
}

/// Downloads the update found by the last check, then installs it and starts the new bar.
/// The old bar gives the screen space back first: the Windows installer ends this process
/// without a normal quit, so nothing after `install` would run.
#[tauri::command]
#[specta::specta]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("An update is already being installed.".into());
    }
    let result = run_install(&app).await;
    // Only reached when something went wrong (a good install ends the process), so the user can try again.
    INSTALLING.store(false, Ordering::SeqCst);
    result
}

async fn run_install(app: &AppHandle) -> Result<(), String> {
    // A copy: the update stays available if the download fails (offline, a dropped connection).
    let update = app
        .state::<Pending>()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .ok_or("There is no update to install. Check for updates first.")?;
    let bytes = update
        .download(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    crate::shutdown_services();
    if let Err(error) = update.install(bytes) {
        // The services are already stopped, so start the bar fresh instead of leaving half of it.
        log::error!("updater: install failed: {error}");
        app.restart();
    }
    app.restart()
}
