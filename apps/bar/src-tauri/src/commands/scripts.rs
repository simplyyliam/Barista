//! Commands of the "Script items" page and the trust dialog (v0.9). Internal: for the bar's own
//! UI, not part of the SDK or the protocol. Each setting goes through `settings::change`, which
//! clamps and saves it and tells every window, then the runner applies it.

use super::settings::change;
use crate::scripts::{self, ScriptInfo, TrustRequest};

/// A script id as the runner makes them: lowercase letters, digits, `-`, `_`, `.`. Anything else
/// can't be a script, so it never reaches the settings file.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
}

/// Every script in the folder with its state. Call once when the page opens; `scriptsChanged`
/// keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_scripts() -> Vec<ScriptInfo> {
    scripts::list()
}

/// The scripts waiting for the user's yes, oldest file name first. `scriptTrustRequested` keeps it fresh.
#[tauri::command]
#[specta::specta]
pub fn get_script_trust_requests() -> Vec<TrustRequest> {
    scripts::trust_requests()
}

/// Turns every script item on or off. Off stops all runs and removes the items.
#[tauri::command]
#[specta::specta]
pub fn set_scripts_enabled(app: tauri::AppHandle, on: bool) {
    change(&app, |s| s.scripts.enabled = on);
    scripts::apply_settings();
}

/// The interval for scripts whose file name has none (1 s to 30 days).
#[tauri::command]
#[specta::specta]
pub fn set_scripts_default_interval(app: tauri::AppHandle, secs: u32) {
    change(&app, |s| s.scripts.default_interval_secs = secs);
    scripts::apply_settings();
}

/// Turns one script on or off. Turning on a script the user said no to asks again.
#[tauri::command]
#[specta::specta]
pub fn set_script_enabled(app: tauri::AppHandle, id: String, on: bool) -> Result<(), String> {
    if !valid_id(&id) {
        return Err("not a script id".into());
    }
    change(&app, |s| s.scripts.per_script.entry(id).or_default().enabled = on);
    scripts::apply_settings();
    Ok(())
}

/// A script's own interval in seconds, or `null` for the default. Has no effect when the file
/// name sets one (`clock.10s.mjs`).
#[tauri::command]
#[specta::specta]
pub fn set_script_interval(app: tauri::AppHandle, id: String, secs: Option<u32>) -> Result<(), String> {
    if !valid_id(&id) {
        return Err("not a script id".into());
    }
    change(&app, |s| s.scripts.per_script.entry(id).or_default().interval_secs = secs);
    scripts::apply_settings();
    Ok(())
}

/// The user answered a trust request. `sha256` is the hash from the request: the answer only
/// counts if the file still has it. `allow = false` switches the script off.
#[tauri::command]
#[specta::specta]
pub fn answer_script_trust(id: String, sha256: String, allow: bool) -> Result<(), String> {
    if !valid_id(&id) {
        return Err("not a script id".into());
    }
    scripts::answer_trust(&id, &sha256, allow)
}

/// Opens `%APPDATA%\Barista\scripts` in Explorer.
#[tauri::command]
#[specta::specta]
pub fn open_scripts_folder() {
    scripts::open_folder();
}

#[cfg(test)]
mod tests {
    use super::valid_id;

    #[test]
    fn only_runner_style_ids_pass() {
        assert!(valid_id("system-monitor.ps1"));
        assert!(valid_id("a_b-1.py"));
        for bad in ["", "A.py", "../x", "a/b", "a b", r"a\b", &"x".repeat(81)] {
            assert!(!valid_id(bad), "{bad}");
        }
    }
}
