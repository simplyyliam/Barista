//! Now playing: title, artist, cover and the play / next / previous buttons of whatever app has
//! the system media session (ADR 0020). `source` says where the track came from, so a later
//! release can add the Spotify web connection next to the system one.

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    /// False when no app is playing anything: the other fields mean nothing then.
    pub available: bool,
    /// `"system"` (Windows media controls). Reserved for more sources later.
    pub source: String,
    /// The player's name, like "Spotify".
    pub app_name: String,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub can_next: bool,
    pub can_previous: bool,
    /// The cover as a data URL (JPEG or PNG only), if the app gave one.
    pub cover: Option<String>,
}

pub fn service() -> &'static Service<Media> {
    static S: OnceLock<Service<Media>> = OnceLock::new();
    S.get_or_init(|| Service::new(Media::default()))
}

/// "Spotify.exe" -> "Spotify"; "SpotifyAB.SpotifyMusic_x!Spotify" -> "Spotify".
pub fn app_name(app_id: &str) -> String {
    let id = app_id.rsplit('!').next().unwrap_or(app_id);
    let id = id.strip_suffix(".exe").unwrap_or(id);
    id.rsplit('.').next().unwrap_or(id).to_string()
}

/// Longest title or artist we pass on (a player could send anything).
const MAX_TEXT: usize = 200;

fn clip(text: &str) -> String {
    text.chars().take(MAX_TEXT).collect()
}

pub fn build(
    app_id: &str,
    title: &str,
    artist: &str,
    playing: bool,
    can_next: bool,
    can_previous: bool,
    cover: Option<&[u8]>,
) -> Media {
    // A session with no title is a player that has nothing loaded.
    if title.trim().is_empty() {
        return Media::default();
    }
    Media {
        available: true,
        source: "system".into(),
        app_name: clip(&app_name(app_id)),
        title: clip(title),
        artist: clip(artist),
        playing,
        can_next,
        can_previous,
        cover: cover.and_then(|b| super::user::avatar_data_url(b, 2 * 1024 * 1024)),
    }
}

/// Asks for a fresh read. Never blocks: the WinRT reads (and the cover) wait on other processes, so
/// they run on one worker thread, and a burst of events becomes one read. Calling this from a
/// command (the panel opening) used to freeze the click that opened it.
pub fn refresh() {
    #[cfg(windows)]
    if let Some(tx) = WORKER.get() {
        let _ = tx.send(());
    }
}

#[cfg(windows)]
static WORKER: std::sync::OnceLock<std::sync::mpsc::Sender<()>> = std::sync::OnceLock::new();

#[cfg(windows)]
fn read_and_publish() {
    if !super::control_center_active() {
        return;
    }
    let media = match crate::platform::windows::media::read() {
        Some(r) => build(&r.app_id, &r.title, &r.artist, r.playing, r.can_next, r.can_previous, r.cover.as_deref()),
        None => Media::default(),
    };
    super::publish_gated(service(), media);
}

pub fn start() {
    #[cfg(windows)]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        if WORKER.set(tx).is_err() {
            return;
        }
        let spawned = std::thread::Builder::new().name("barista-media-read".into()).spawn(move || {
            while rx.recv().is_ok() {
                while rx.try_recv().is_ok() {}
                read_and_publish();
            }
        });
        if spawned.is_err() {
            eprintln!("barista: media reader not started");
            return;
        }
        crate::platform::windows::media::start(refresh);
    }
}

pub fn play_pause() {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    crate::platform::windows::media::play_pause();
}

pub fn next() {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    crate::platform::windows::media::next();
}

pub fn previous() {
    if !super::control_center_active() {
        return;
    }
    #[cfg(windows)]
    crate::platform::windows::media::previous();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_from_app_ids() {
        assert_eq!(app_name("Spotify.exe"), "Spotify");
        assert_eq!(app_name("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"), "Spotify");
        assert_eq!(app_name("chrome.exe"), "chrome");
    }

    #[test]
    fn no_title_means_nothing_playing() {
        assert!(!build("x.exe", "  ", "a", true, true, true, None).available);
    }

    #[test]
    fn long_text_is_clipped_and_bad_covers_dropped() {
        let long = "a".repeat(1000);
        let m = build("x.exe", &long, &long, true, false, false, Some(b"not a picture"));
        assert_eq!(m.title.len(), MAX_TEXT);
        assert_eq!(m.artist.len(), MAX_TEXT);
        assert!(m.cover.is_none());
    }
}
