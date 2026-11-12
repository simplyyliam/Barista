//! Wakes a callback when a registry key changes (`RegNotifyChangeKeyValue`), and reads one DWORD.
//! The thread sleeps in `WaitForSingleObject`; nothing polls.

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegCloseKey, RegGetValueW, RegNotifyChangeKeyValue, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE,
    KEY_NOTIFY, REG_NOTIFY_CHANGE_LAST_SET, RRF_RT_REG_DWORD,
};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject, INFINITE};

/// The DWORD in the default value of `HKLM\<subkey>`, or `None` if the key or value is missing.
pub fn read_default_dword(subkey: &str) -> Option<u32> {
    read_default_dword_in(HKEY_LOCAL_MACHINE, subkey)
}

fn read_default_dword_in(root: HKEY, subkey: &str) -> Option<u32> {
    let mut value = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: `value` and `size` live for the call and match the requested DWORD type.
    let status = unsafe {
        RegGetValueW(
            root,
            &HSTRING::from(subkey),
            PCWSTR::null(),
            RRF_RT_REG_DWORD,
            None,
            Some(std::ptr::from_mut(&mut value).cast()),
            Some(&mut size),
        )
    };
    (status == ERROR_SUCCESS).then_some(value)
}

/// Calls `on_change` on a new thread each time something under `HKLM\<subkey>` changes.
/// Returns false if the key can't be watched.
pub fn watch(subkey: &str, on_change: fn()) -> bool {
    watch_in(HKEY_LOCAL_MACHINE, subkey, on_change)
}

fn watch_in(root: HKEY, subkey: &str, on_change: fn()) -> bool {
    let mut key = HKEY::default();
    // SAFETY: plain call; the key is closed again if the thread can't start.
    let opened = unsafe {
        RegOpenKeyExW(root, &HSTRING::from(subkey), None, KEY_NOTIFY, &mut key)
    };
    if opened != ERROR_SUCCESS {
        return false;
    }
    // SAFETY: auto-reset event, unnamed; closed when the thread ends.
    let Ok(event) = (unsafe { CreateEventW(None, false, false, w!("")) }) else {
        // SAFETY: closes the key opened above.
        unsafe {
            let _ = RegCloseKey(key);
        }
        return false;
    };
    // HKEY and HANDLE are plain numbers: pass them to the thread as such.
    let (k, e) = (key.0 as usize, event.0 as usize);
    let spawned = std::thread::Builder::new()
        .name("barista-regwatch".into())
        .spawn(move || {
            let key = HKEY(k as _);
            let event = windows::Win32::Foundation::HANDLE(e as _);
            loop {
                // SAFETY: both handles stay open until the loop ends. The notification is one-shot,
                // so it is armed again each round, before reading, so no change is lost.
                let armed = unsafe {
                    RegNotifyChangeKeyValue(key, true, REG_NOTIFY_CHANGE_LAST_SET, Some(event), true)
                };
                if armed != ERROR_SUCCESS {
                    break;
                }
                // SAFETY: waits on the event armed above.
                unsafe { WaitForSingleObject(event, INFINITE) };
                on_change();
            }
            // SAFETY: closes what was opened in `watch`.
            unsafe {
                let _ = RegCloseKey(key);
                let _ = CloseHandle(event);
            }
        });
    if spawned.is_err() {
        // SAFETY: the thread never ran, so these are still ours to close.
        unsafe {
            let _ = RegCloseKey(key);
            let _ = CloseHandle(event);
        }
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{Duration, Instant};
    use windows::Win32::System::Registry::HKEY_CURRENT_USER;

    static FIRED: AtomicU32 = AtomicU32::new(0);
    const KEY: &str = r"Software\BaristaWatchTest";

    fn reg(args: &[&str]) {
        let _ = Command::new("reg").args(args).output();
    }

    /// The same code path as the airplane mode watch, on a key we may write without admin rights.
    #[test]
    #[ignore = "writes a temporary HKCU key"]
    fn a_registry_change_wakes_the_watcher() {
        reg(&["add", &format!(r"HKCU\{KEY}"), "/ve", "/t", "REG_DWORD", "/d", "0", "/f"]);
        assert!(watch_in(HKEY_CURRENT_USER, KEY, || {
            FIRED.fetch_add(1, Ordering::SeqCst);
        }));
        std::thread::sleep(Duration::from_millis(300));
        let before = FIRED.load(Ordering::SeqCst);
        reg(&["add", &format!(r"HKCU\{KEY}"), "/ve", "/t", "REG_DWORD", "/d", "1", "/f"]);
        let started = Instant::now();
        while FIRED.load(Ordering::SeqCst) == before && started.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(10));
        }
        println!("woke after {:?}", started.elapsed());
        assert!(FIRED.load(Ordering::SeqCst) > before, "no wake-up");
        assert_eq!(read_default_dword_in(HKEY_CURRENT_USER, KEY), Some(1));
        reg(&["delete", &format!(r"HKCU\{KEY}"), "/f"]);
    }
}
