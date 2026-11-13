//! "Now playing" through the system media controls (SMTC): the same session Windows shows in its
//! volume flyout, so Spotify, browsers and media players all work without per-app code.
//!
//! Windows calls our handlers on its own threads; they only call `on_change`, and the service
//! reads everything again. Nothing polls.

use std::sync::{Mutex, OnceLock};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::{DataReader, IRandomAccessStreamReference};

/// A track picture bigger than this is not read at all (rule 3: never trust what an app sends).
const MAX_COVER: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct Raw {
    pub app_id: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub cover: Option<Vec<u8>>,
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

static MANAGER: OnceLock<Manager> = OnceLock::new();
static HOOKED: Mutex<Option<Hooked>> = Mutex::new(None);
static ON_CHANGE: OnceLock<fn()> = OnceLock::new();

fn notify() {
    if let Some(f) = ON_CHANGE.get() {
        f();
    }
}

/// Hooks the current session (dropping the previous hook), then tells the service.
fn rehook() {
    let Some(manager) = MANAGER.get() else { return };
    let session = manager.GetCurrentSession().ok();
    {
        let mut slot = HOOKED.lock().unwrap_or_else(|e| e.into_inner());
        *slot = session.and_then(|session| {
            let a = session
                .MediaPropertiesChanged(&TypedEventHandler::new(|_, _| {
                    notify();
                    Ok(())
                }))
                .ok()?;
            let b = session
                .PlaybackInfoChanged(&TypedEventHandler::new(|_, _| {
                    notify();
                    Ok(())
                }))
                .ok()?;
            Some(Hooked { session, tokens: (a, b) })
        });
    }
    notify();
}

/// Starts listening. `on_change` runs (on a Windows thread) whenever the track, the playing state
/// or the current player changes.
pub fn start(on_change: fn()) {
    let _ = ON_CHANGE.set(on_change);
    let spawned = std::thread::Builder::new().name("barista-media".into()).spawn(|| {
        let Ok(op) = Manager::RequestAsync() else { return };
        let Ok(manager) = op.join() else { return };
        let _ = manager.CurrentSessionChanged(&TypedEventHandler::new(|_, _| {
            rehook();
            Ok(())
        }));
        let _ = MANAGER.set(manager);
        rehook();
    });
    if spawned.is_err() {
        eprintln!("barista: media controls not started");
    }
}

/// The current track, or `None` when no app has a media session.
pub fn read() -> Option<Raw> {
    let session = MANAGER.get()?.GetCurrentSession().ok()?;
    let props = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let info = session.GetPlaybackInfo().ok()?;
    let controls = info.Controls().ok()?;
    Some(Raw {
        app_id: session.SourceAppUserModelId().ok()?.to_string(),
        title: props.Title().ok()?.to_string(),
        artist: props.Artist().map(|a| a.to_string()).unwrap_or_default(),
        playing: info.PlaybackStatus().ok()? == Status::Playing,
        can_next: controls.IsNextEnabled().unwrap_or(false),
        can_previous: controls.IsPreviousEnabled().unwrap_or(false),
        cover: props.Thumbnail().ok().and_then(|t| read_picture(&t)),
    })
}

fn read_picture(reference: &IRandomAccessStreamReference) -> Option<Vec<u8>> {
    let stream = reference.OpenReadAsync().ok()?.join().ok()?;
    let size = stream.Size().ok()?;
    if size == 0 || size > MAX_COVER {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).ok()?).ok()?;
    reader.LoadAsync(size as u32).ok()?.join().ok()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    Some(bytes)
}

fn with_session(f: impl FnOnce(&Session) -> windows::core::Result<()>) {
    if let Some(session) = MANAGER.get().and_then(|m| m.GetCurrentSession().ok()) {
        let _ = f(&session);
    }
}

pub fn play_pause() {
    with_session(|s| s.TryTogglePlayPauseAsync().map(|_| ()));
}

pub fn next() {
    with_session(|s| s.TrySkipNextAsync().map(|_| ()));
}

pub fn previous() {
    with_session(|s| s.TrySkipPreviousAsync().map(|_| ()));
}
