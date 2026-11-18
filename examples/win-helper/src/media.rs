//! Now playing through the system media controls (SMTC): the session Windows shows in its volume
//! flyout, so Spotify, browsers and media players all work without per-app code. Windows calls
//! our handlers on its own threads; they only send a message, and the worker reads everything
//! again. Nothing polls.

use super::Out;
use serde_json::{json, Value};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};

enum Msg {
    /// The track, the playing state or the current player changed.
    Changed,
    /// The current player is a different one: listen to it.
    Rehook,
    Command(Value),
}

/// The session we listen to, with the tokens to stop listening again.
struct Hooked {
    session: Session,
    tokens: (i64, i64),
}

impl Drop for Hooked {
    fn drop(&mut self) {
        let _ = self.session.RemoveMediaPropertiesChanged(self.tokens.0);
        let _ = self.session.RemovePlaybackInfoChanged(self.tokens.1);
    }
}

fn hook(manager: &Manager, tx: &Sender<Msg>) -> Option<Hooked> {
    let session = manager.GetCurrentSession().ok()?;
    let a = {
        let tx = tx.clone();
        session
            .MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
                let _ = tx.send(Msg::Changed);
                Ok(())
            }))
            .ok()?
    };
    let b = {
        let tx = tx.clone();
        session
            .PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
                let _ = tx.send(Msg::Changed);
                Ok(())
            }))
            .ok()?
    };
    Some(Hooked { session, tokens: (a, b) })
}

fn read(manager: &Manager) -> Value {
    let none = json!({"type": "media", "active": false, "playing": false, "title": "", "artist": "",
                      "app": "", "canPrevious": false, "canNext": false});
    let Ok(session) = manager.GetCurrentSession() else { return none };
    let Some(props) = session.TryGetMediaPropertiesAsync().ok().and_then(|o| o.join().ok()) else {
        return none;
    };
    let Ok(info) = session.GetPlaybackInfo() else { return none };
    let controls = info.Controls().ok();
    let flag = |f: fn(&windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackControls) -> windows::core::Result<bool>| {
        controls.as_ref().and_then(|c| f(c).ok()).unwrap_or(false)
    };
    json!({
        "type": "media",
        "active": true,
        "playing": info.PlaybackStatus().is_ok_and(|s| s == Status::Playing),
        "title": props.Title().map(|t| t.to_string()).unwrap_or_default(),
        "artist": props.Artist().map(|t| t.to_string()).unwrap_or_default(),
        "app": session.SourceAppUserModelId().map(|t| t.to_string()).unwrap_or_default(),
        "canPrevious": flag(|c| c.IsPreviousEnabled()),
        "canNext": flag(|c| c.IsNextEnabled()),
    })
}

/// Starts the worker thread and returns where commands go.
pub fn start(out: Arc<Out>) -> Sender<Value> {
    let (cmd_tx, cmd_rx) = channel::<Value>();
    let (tx, rx) = channel::<Msg>();
    let forward = tx.clone();
    std::thread::spawn(move || {
        while let Ok(v) = cmd_rx.recv() {
            if forward.send(Msg::Command(v)).is_err() {
                break;
            }
        }
        // stdin closed: the parent is gone.
        std::process::exit(0);
    });
    std::thread::spawn(move || {
        let Some(manager) = Manager::RequestAsync().ok().and_then(|op| op.join().ok()) else {
            eprintln!("win-helper: media controls not available");
            out.emit(json!({"type": "media", "active": false, "playing": false, "title": "",
                            "artist": "", "app": "", "canPrevious": false, "canNext": false}));
            return;
        };
        {
            let tx = tx.clone();
            let _ = manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
                let _ = tx.send(Msg::Rehook);
                Ok(())
            }));
        }
        let mut hooked = hook(&manager, &tx);
        out.emit(read(&manager));
        while let Ok(msg) = rx.recv() {
            match msg {
                Msg::Rehook => {
                    drop(hooked.take());
                    hooked = hook(&manager, &tx);
                }
                Msg::Changed => {}
                Msg::Command(v) => {
                    if let Ok(session) = manager.GetCurrentSession() {
                        let _ = match v.get("cmd").and_then(Value::as_str) {
                            Some("playPause") => session.TryTogglePlayPauseAsync().map(|_| ()),
                            Some("next") => session.TrySkipNextAsync().map(|_| ()),
                            Some("previous") => session.TrySkipPreviousAsync().map(|_| ()),
                            _ => Ok(()),
                        };
                    }
                }
            }
            out.emit(read(&manager));
        }
    });
    cmd_tx
}
