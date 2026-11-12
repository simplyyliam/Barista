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
    let mut value = 0u32;
    let mut size = size_of::<u32>() as u32;
    // SAFETY: `value` and `size` live for the call and match the requested DWORD type.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
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
    let mut key = HKEY::default();
    // SAFETY: plain call; the key is closed again if the thread can't start.
    let opened = unsafe {
        RegOpenKeyExW(HKEY_LOCAL_MACHINE, &HSTRING::from(subkey), None, KEY_NOTIFY, &mut key)
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
