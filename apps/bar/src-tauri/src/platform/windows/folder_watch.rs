//! Calls `on_change` whenever something in a folder is written. One thread sleeps in
//! `WaitForSingleObject` until Windows says so, so it costs nothing while idle (CLAUDE.md rule 9).
//! One change can fire it a few times; the receiver is expected to wait for the burst to settle.

use std::path::PathBuf;
use windows::core::HSTRING;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    FindFirstChangeNotificationW, FindNextChangeNotification, FILE_NOTIFY_CHANGE_FILE_NAME,
    FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE,
};
use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};

/// Starts the watcher thread. It lives as long as the app.
pub fn start(dir: PathBuf, on_change: impl Fn() + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("folder-watch".into())
        .spawn(move || {
            // SAFETY: the handle is made, waited on and re-armed only on this thread, and the
            // thread never ends, so it is never closed while in use.
            unsafe {
                let Ok(handle) = FindFirstChangeNotificationW(
                    &HSTRING::from(dir.as_os_str()),
                    false,
                    FILE_NOTIFY_CHANGE_FILE_NAME
                        | FILE_NOTIFY_CHANGE_LAST_WRITE
                        | FILE_NOTIFY_CHANGE_SIZE,
                ) else {
                    eprintln!("barista: could not watch {}", dir.display());
                    return;
                };
                loop {
                    if WaitForSingleObject(HANDLE(handle.0), INFINITE).0 != 0 {
                        return; // a wait error: give up rather than spin
                    }
                    let _ = FindNextChangeNotification(handle);
                    on_change();
                }
            }
        });
    if let Err(e) = spawned {
        eprintln!("barista: could not start the folder watcher: {e}");
    }
}
