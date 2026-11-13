//! Debug builds only: the "debug" window (Ctrl+Alt+Shift+D) where the UI shows bar items with
//! made-up states (charging, mute, 12/24 h, ...), so nothing needs to happen on the real machine.

use crate::window_radius::{self, Mode};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "debug";

/// Opens the debug window, or brings it to the front if it is already open.
pub fn open(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.unminimize();
        let _ = window.set_focus();
        return;
    }
    // Building a window inside a Win32 message handler can deadlock Tauri, so do it elsewhere.
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = build(&app) {
            eprintln!("barista: could not open the debug window: {e}");
        }
    });
}

fn build(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Barista debug")
        .inner_size(760.0, 600.0)
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
    window_radius::register(app, LABEL, Mode::Normal)?;
    Ok(())
}
