//! Reads and removes Windows toasts through `UserNotificationListener` (ADR 0017).
//!
//! WinRT wants its own thread and apartment, so one worker owns the listener. Other threads send
//! it jobs; it waits a moment for a burst to settle (a few toasts at once), runs them in order,
//! reads everything once and calls `publish`. Nothing here polls: jobs come from the folder watcher.

use crate::system::notifications::{Access, RawToast};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use windows::UI::Notifications::{
    KnownNotificationBindings,
    Management::{UserNotificationListener, UserNotificationListenerAccessStatus},
    NotificationKinds, UserNotification,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

pub enum Job {
    Refresh,
    Remove(u32),
    Clear,
}

static TX: OnceLock<Sender<Job>> = OnceLock::new();

/// A burst of toasts becomes one read: wait this long after the last job, but never more than
/// `MAX_WAIT` in total.
const SETTLE: Duration = Duration::from_millis(250);
const MAX_WAIT: Duration = Duration::from_secs(1);
/// Seconds from 1601 (Windows time) to 1970 (Unix time).
const UNIX_EPOCH_SECS: i64 = 11_644_473_600;

pub fn start(publish: fn(Vec<RawToast>, Access)) {
    let (tx, rx) = channel::<Job>();
    if TX.set(tx).is_err() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("notifications".into())
        .spawn(move || {
            // SAFETY: plain COM init for this thread; the WinRT calls below need an apartment.
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
            }
            let listener = UserNotificationListener::Current().ok();
            if let Some(l) = &listener {
                // A desktop app gets no prompt, but this is what registers the request.
                let _ = l.RequestAccessAsync().and_then(|op| op.join());
            }
            while let Ok(first) = rx.recv() {
                let mut jobs = vec![first];
                let started = Instant::now();
                while started.elapsed() < MAX_WAIT {
                    let left = MAX_WAIT.saturating_sub(started.elapsed()).min(SETTLE);
                    match rx.recv_timeout(left) {
                        Ok(job) => jobs.push(job),
                        Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
                let Some(l) = &listener else {
                    publish(Vec::new(), Access::Unknown);
                    continue;
                };
                for job in &jobs {
                    let _ = match job {
                        Job::Remove(id) => l.RemoveNotification(*id),
                        Job::Clear => l.ClearNotifications(),
                        Job::Refresh => Ok(()),
                    };
                }
                let (raws, access) = read(l);
                publish(raws, access);
            }
        });
    if let Err(e) = spawned {
        eprintln!("barista: could not start the notifications worker: {e}");
    }
}

pub fn send(job: Job) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(job);
    }
}

fn read(l: &UserNotificationListener) -> (Vec<RawToast>, Access) {
    let access = match l.GetAccessStatus() {
        Ok(UserNotificationListenerAccessStatus::Allowed) => Access::Allowed,
        Ok(UserNotificationListenerAccessStatus::Denied) => Access::Denied,
        _ => Access::Unknown,
    };
    if access != Access::Allowed {
        return (Vec::new(), access);
    }
    let list = l
        .GetNotificationsAsync(NotificationKinds::Toast)
        .and_then(|op| op.join());
    let Ok(list) = list else {
        return (Vec::new(), Access::Unknown);
    };
    let raws = list.into_iter().filter_map(|n| raw_toast(&n).ok()).collect();
    (raws, access)
}

fn raw_toast(n: &UserNotification) -> windows::core::Result<RawToast> {
    let (mut app_id, mut app_name) = (String::new(), String::new());
    if let Ok(app) = n.AppInfo() {
        app_id = app.AppUserModelId()?.to_string();
        app_name = app.DisplayInfo()?.DisplayName()?.to_string();
    }
    // Old-style toast templates have no ToastGeneric binding: no texts here, the XML fills in.
    let texts = (|| -> windows::core::Result<Vec<String>> {
        let visual = n.Notification()?.Visual()?;
        let generic = visual.GetBinding(&KnownNotificationBindings::ToastGeneric()?)?;
        generic
            .GetTextElements()?
            .into_iter()
            .map(|t| Ok(t.Text()?.to_string()))
            .collect()
    })()
    .unwrap_or_default();
    let secs = n.CreationTime()?.UniversalTime / 10_000_000 - UNIX_EPOCH_SECS;
    Ok(RawToast {
        id: n.Id()?,
        app_id,
        app_name,
        texts,
        arrived: secs.clamp(0, u32::MAX as i64) as u32,
    })
}
