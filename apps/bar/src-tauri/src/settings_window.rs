//! The settings window (design: docs/design/settings-window.png). One instance, opened from the
//! brand menu or the `openSettingsWindow` command. Built like the debug window, on its own thread
//! (building inside a Win32 message handler can deadlock Tauri).

use crate::window_radius::{self, Mode};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "settings";

static BUILDING: AtomicBool = AtomicBool::new(false);

/// Opens the window, or brings it to the front if it is already open.
pub fn open(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    if BUILDING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = build(&app) {
            eprintln!("barista: could not open the settings window: {e}");
        }
        BUILDING.store(false, Ordering::SeqCst);
    });
}

fn build(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Barista Settings")
        // No frame: the page draws its own titlebar (features/settings/titlebar.tsx), so it can
        // be black to the edge. The permissions are in capabilities/settings.json.
        .decorations(false)
        .background_color(tauri::window::Color(0, 0, 0, 255))
        .inner_size(1052.0, 748.0)
        .min_inner_size(820.0, 560.0)
        .center();
    // WebView2 only lets windows that share a data folder use the same browser arguments.
    if let Some(args) = app
        .config()
        .app
        .windows
        .first()
        .and_then(|w| w.additional_browser_args.clone())
    {
        builder = builder.additional_browser_args(&args);
    }
    builder.build()?;
    // Tracked: this window can be maximized or snapped, so its corner radius follows.
    window_radius::register(app, LABEL, Mode::Tracked)?;
    Ok(())
}
