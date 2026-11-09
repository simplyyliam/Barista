//! Wakes us when the desktop wallpaper changes, whichever way it was changed.
//!
//! `WM_SETTINGCHANGE` (see `window.rs`) only comes from apps that use the old
//! `SystemParametersInfo` call. The Settings app (and slideshows) change the wallpaper through
//! other APIs and may not send it, so we also watch what they write:
//!
//! ```text
//! HKCU\Control Panel\Desktop                                  the "Wallpaper" value
//! HKCU\...\Explorer\Wallpapers                                the wallpaper history
//! %APPDATA%\Microsoft\Windows\Themes                          the TranscodedWallpaper file
//! ```
//!
//! One thread sleeps in `WaitForMultipleObjects` until Windows signals one of them, so it costs
//! nothing while nothing changes (CLAUDE.md rule 9). It may fire a few times for one change;
//! the appearance worker waits for the burst to settle and only reports a real tone change.

use windows::core::HSTRING;
use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
use windows::Win32::Storage::FileSystem::{
    FindFirstChangeNotificationW, FindNextChangeNotification, FILE_NOTIFY_CHANGE_LAST_WRITE,
    FILE_NOTIFY_CHANGE_SIZE,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_NOTIFY,
    REG_NOTIFY_CHANGE_LAST_SET,
};
use windows::Win32::System::Threading::{
    CreateEventW, ResetEvent, WaitForMultipleObjects, INFINITE,
};

const KEYS: [&str; 2] = [
    r"Control Panel\Desktop",
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\Wallpapers",
];

/// Starts the watcher thread. It lives as long as the app, so there is nothing to stop.
pub fn start(on_change: fn()) {
    let spawned = std::thread::Builder::new()
        .name("wallpaper-watch".into())
        .spawn(move || run(on_change));
    if let Err(e) = spawned {
        eprintln!("barista: could not start the wallpaper watcher: {e}");
    }
}

fn run(on_change: fn()) {
    // SAFETY: plain Win32 calls. Every handle below is created here and used only on this thread,
    // and the thread never ends, so none is closed while still in use.
    unsafe {
        // One manual-reset event per registry key; the folder gets its own change handle.
        let mut keys: Vec<(HKEY, HANDLE)> = Vec::new();
        for path in KEYS {
            let mut key = HKEY::default();
            let status =
                RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path), None, KEY_NOTIFY, &mut key);
            if status != ERROR_SUCCESS {
                continue;
            }
            match CreateEventW(None, true, false, None) {
                Ok(event) => keys.push((key, event)),
                Err(_) => {
                    let _ = RegCloseKey(key);
                }
            }
        }
        let folder = std::env::var("APPDATA")
            .ok()
            .map(|a| format!("{a}\\Microsoft\\Windows\\Themes"))
            .and_then(|dir| {
                FindFirstChangeNotificationW(
                    &HSTRING::from(dir),
                    false,
                    FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_SIZE,
                )
                .ok()
            });

        let mut waits: Vec<HANDLE> = keys.iter().map(|(_, event)| *event).collect();
        if let Some(f) = &folder {
            waits.push(HANDLE(f.0));
        }
        if waits.is_empty() {
            return;
        }

        loop {
            // Arm every key again (a registry notification fires once), then wait.
            for (key, event) in &keys {
                let _ = ResetEvent(*event);
                let _ = RegNotifyChangeKeyValue(*key, false, REG_NOTIFY_CHANGE_LAST_SET, Some(*event), true);
            }
            let signalled = WaitForMultipleObjects(&waits, false, INFINITE);
            let index = signalled.0 as usize; // WAIT_OBJECT_0 is 0
            if index >= waits.len() {
                return; // a wait error: give up rather than spin
            }
            if folder.is_some() && index == waits.len() - 1 {
                if let Some(f) = &folder {
                    let _ = FindNextChangeNotification(*f);
                }
            }
            on_change();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{DesktopWallpaper, IDesktopWallpaper};

    static HITS: AtomicUsize = AtomicUsize::new(0);

    /// Re-applies the CURRENT wallpaper through `IDesktopWallpaper::SetWallpaper` (what the Settings
    /// app uses; nothing visible changes) and checks the watcher noticed. This process has no
    /// window, so `WM_SETTINGCHANGE` cannot be what woke it:
    /// `cargo test wallpaper_watch_live -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn wallpaper_watch_live() {
        start(|| {
            HITS.fetch_add(1, Ordering::SeqCst);
        });
        std::thread::sleep(Duration::from_millis(300)); // let it arm
        // SAFETY: COM calls on this thread after initialising it; the path comes from Windows.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();
            let w: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL).unwrap();
            let current = w.GetWallpaper(windows::core::PCWSTR::null()).unwrap();
            w.SetWallpaper(windows::core::PCWSTR::null(), windows::core::PCWSTR(current.0)).unwrap();
        }
        let started = Instant::now();
        while HITS.load(Ordering::SeqCst) == 0 && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(50));
        }
        eprintln!("watcher fired {} time(s), first after ~{:?}", HITS.load(Ordering::SeqCst), started.elapsed());
        assert!(HITS.load(Ordering::SeqCst) > 0, "the watcher did not notice the change");
    }
}
