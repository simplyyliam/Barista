//! Reads and watches the settings of Windhawk's "custom-corner-radius" mod in the registry.
//!
//! ```text
//! HKLM\SOFTWARE\Windhawk\Engine\Mods                              one subkey per installed mod
//! HKLM\SOFTWARE\Windhawk\Engine\Mods\custom-corner-radius         DWORD  Disabled  (0 = on)
//! HKLM\SOFTWARE\Windhawk\Engine\Mods\custom-corner-radius\Settings
//!                                                                 DWORD  radius    (-1 = keep original)
//!                                                                 SZ     roundMaximizedAndSnapped
//!                                                                        ("none", "snapped", "snappedAndMaximized")
//! ```
//!
//! Everything here only reads. Whether a value means "active" and what radius follows from it is
//! decided in `crate::radius`, which has no Win32 in it.

use crate::radius::{settings_from_values, ModSettings};
use std::io;
use std::thread::JoinHandle;
use std::time::Duration;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_MORE_DATA, ERROR_SUCCESS, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW, RegQueryValueExW, HKEY,
    HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY, REG_DWORD, REG_NOTIFY_CHANGE_LAST_SET,
    REG_NOTIFY_CHANGE_NAME, REG_NOTIFY_FILTER, REG_SZ, REG_VALUE_TYPE,
};
use windows::Win32::System::Threading::{
    CreateEventW, ResetEvent, SetEvent, WaitForMultipleObjects, WaitForSingleObject, INFINITE,
};

/// Under HKLM. The `Mods` key holds one subkey per installed mod.
const MODS_KEY: &str = r"SOFTWARE\Windhawk\Engine\Mods";
const MOD_NAME: &str = "custom-corner-radius";

/// Windhawk writes the values of one settings change one by one. Waiting this long after the
/// first notification means we read once, when all of them are there, not halfway.
const DEBOUNCE: Duration = Duration::from_millis(50);

/// An open registry key, closed when it goes out of scope.
struct RegKey(HKEY);

impl RegKey {
    /// Opens `path` under `root` for reading. `None` when it does not exist or we may not read it.
    fn open(root: HKEY, path: &str) -> Option<RegKey> {
        let mut key = HKEY::default();
        // SAFETY: `key` is a valid out-pointer and the path string outlives the call.
        // KEY_WOW64_64KEY asks for the real 64-bit view, where Windhawk writes, even if this
        // process were ever built as 32-bit.
        let status = unsafe {
            RegOpenKeyExW(
                root,
                &HSTRING::from(path),
                None,
                KEY_READ | KEY_WOW64_64KEY,
                &mut key,
            )
        };
        (status == ERROR_SUCCESS).then_some(RegKey(key))
    }

    /// The raw bytes and type of a value, or `None` when it is missing.
    fn query(&self, name: &str) -> Option<(REG_VALUE_TYPE, Vec<u8>)> {
        let name = HSTRING::from(name);
        // The value can grow between asking for its size and reading it, so retry a few times.
        for _ in 0..4 {
            let mut kind = REG_VALUE_TYPE(0);
            let mut len = 0u32;
            // SAFETY: valid out-pointers. With no data pointer Windows only reports the size.
            let status = unsafe {
                RegQueryValueExW(self.0, &name, None, Some(&mut kind), None, Some(&mut len))
            };
            if status != ERROR_SUCCESS {
                return None;
            }
            let mut buffer = vec![0u8; len as usize];
            if len == 0 {
                return Some((kind, buffer));
            }
            let mut read = len;
            // SAFETY: `buffer` holds `read` bytes and stays alive during the call.
            let status = unsafe {
                RegQueryValueExW(
                    self.0,
                    &name,
                    None,
                    Some(&mut kind),
                    Some(buffer.as_mut_ptr()),
                    Some(&mut read),
                )
            };
            if status == ERROR_SUCCESS {
                buffer.truncate(read as usize);
                return Some((kind, buffer));
            }
            if status != ERROR_MORE_DATA {
                return None;
            }
        }
        None
    }

    /// A `REG_DWORD` value. A value of another type counts as missing.
    fn dword(&self, name: &str) -> Option<u32> {
        match self.query(name)? {
            (kind, bytes) if kind == REG_DWORD => Some(u32::from_le_bytes(bytes.try_into().ok()?)),
            _ => None,
        }
    }

    /// A `REG_SZ` value. A value of another type counts as missing.
    fn string(&self, name: &str) -> Option<String> {
        let (kind, bytes) = self.query(name)?;
        if kind != REG_SZ || bytes.len() % 2 != 0 {
            return None;
        }
        let wide: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        // The stored string normally ends with a NUL that is not part of the text.
        let text = wide.split(|&c| c == 0).next().unwrap_or(&[]);
        String::from_utf16(text).ok()
    }
}

impl Drop for RegKey {
    fn drop(&mut self) {
        // SAFETY: the handle came from RegOpenKeyExW and is closed exactly once, here.
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

/// A Win32 event object, closed when it goes out of scope.
struct Event(HANDLE);

// SAFETY: an event handle is a kernel object id; any thread may wait on, set or close it.
unsafe impl Send for Event {}
unsafe impl Sync for Event {}

impl Event {
    /// A manual-reset event: it stays signalled until `reset`.
    fn new() -> io::Result<Event> {
        // SAFETY: no security attributes, no name: a plain private event.
        unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
            .map(Event)
            .map_err(io::Error::other)
    }

    fn set(&self) {
        // SAFETY: valid handle owned by `self`.
        unsafe {
            let _ = SetEvent(self.0);
        }
    }

    fn reset(&self) {
        // SAFETY: valid handle owned by `self`.
        unsafe {
            let _ = ResetEvent(self.0);
        }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        // SAFETY: closed exactly once, here.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// The mod's settings right now, or `None` when it is not active: no Windhawk, mod not
/// installed, `Disabled` set, no radius, or radius -1 ("keep original").
pub fn read_settings() -> Option<ModSettings> {
    read_settings_at(HKEY_LOCAL_MACHINE, MODS_KEY)
}

/// Same, with the registry location given, so tests can use a scratch key under HKCU.
fn read_settings_at(root: HKEY, mods_key: &str) -> Option<ModSettings> {
    // A missing mod key means "not installed": not active.
    let mod_key = RegKey::open(root, &format!(r"{mods_key}\{MOD_NAME}"))?;
    let disabled = mod_key.dword("Disabled");
    let settings = RegKey::open(root, &format!(r"{mods_key}\{MOD_NAME}\Settings"));
    let radius = settings.as_ref().and_then(|key| key.dword("radius"));
    let round = settings
        .as_ref()
        .and_then(|key| key.string("roundMaximizedAndSnapped"));
    settings_from_values(disabled, radius, round.as_deref())
}

/// Stops the watcher thread when dropped.
pub struct Watcher {
    stop: Event,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop.set();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Calls `on_change` whenever the Windhawk mod's registry keys change, and once right away.
/// It runs on a background thread of its own, so it must be quick and may not block.
/// Keep the returned [`Watcher`] alive: dropping it stops the thread.
///
/// There is no polling: the thread sleeps in `WaitForMultipleObjects` until Windows signals
/// `RegNotifyChangeKeyValue`, so it costs nothing while nothing changes (CLAUDE.md rule 9).
pub fn watch(on_change: impl Fn() + Send + 'static) -> io::Result<Watcher> {
    watch_at(HKEY_LOCAL_MACHINE, MODS_KEY, on_change)
}

fn watch_at(
    root: HKEY,
    mods_key: &str,
    on_change: impl Fn() + Send + 'static,
) -> io::Result<Watcher> {
    let stop = Event::new()?;
    let change = Event::new()?;
    // Raw handles are not `Send`; pass them as integers and rebuild them in the thread.
    let root_bits = root.0 as isize;
    let stop_bits = stop.0 .0 as isize;
    let mods_key = mods_key.to_string();
    let thread = std::thread::Builder::new()
        .name("windhawk-watch".into())
        .spawn(move || {
            let root = HKEY(root_bits as *mut _);
            let stop = HANDLE(stop_bits as *mut _);
            watch_loop(root, &mods_key, &change, stop, &on_change);
        })?;
    Ok(Watcher {
        stop,
        thread: Some(thread),
    })
}

fn watch_loop(root: HKEY, mods_key: &str, change: &Event, stop: HANDLE, on_change: &dyn Fn()) {
    loop {
        // Watch the deepest key that exists. Once Windhawk or the mod appears (or goes away) the
        // watched key signals and the next round picks a deeper (or shallower) one.
        let armed = open_deepest(root, mods_key).and_then(|(key, subtree, filter)| {
            change.reset();
            // SAFETY: `key` is open for notification (KEY_READ includes KEY_NOTIFY) and
            // `change` outlives the wait below. `true` = asynchronous: signal the event
            // instead of blocking this call.
            let status =
                unsafe { RegNotifyChangeKeyValue(key.0, subtree, filter, Some(change.0), true) };
            (status == ERROR_SUCCESS).then_some(key)
        });

        // Read only after arming: a change in between would otherwise be lost.
        on_change();

        let Some(_key) = armed else {
            // Could not watch anything (should not happen: SOFTWARE always exists). Try again
            // in a second, unless we are asked to stop.
            // SAFETY: `stop` stays valid until the Watcher is dropped, which joins this thread.
            if unsafe { WaitForSingleObject(stop, 1000) } == WAIT_OBJECT_0 {
                return;
            }
            continue;
        };
        // SAFETY: both handles are valid for the life of this thread (see above).
        let signalled = unsafe { WaitForMultipleObjects(&[change.0, stop], false, INFINITE) };
        if signalled.0 != WAIT_OBJECT_0.0 {
            return; // stop event (index 1) or an error: leave
        }
        // SAFETY: as above. Let a burst of writes finish before the next read.
        if unsafe { WaitForSingleObject(stop, DEBOUNCE.as_millis() as u32) } == WAIT_OBJECT_0 {
            return;
        }
    }
}

/// The key to watch: `mods_key` itself (with everything below it, so `Disabled` and `Settings\*`
/// are covered), or, while it does not exist, its nearest existing parent (only for subkeys being
/// created or deleted, which is cheap).
fn open_deepest(root: HKEY, mods_key: &str) -> Option<(RegKey, bool, REG_NOTIFY_FILTER)> {
    let mut path = mods_key;
    let mut is_mods_key = true;
    loop {
        if let Some(key) = RegKey::open(root, path) {
            return Some(if is_mods_key {
                (
                    key,
                    true,
                    REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET,
                )
            } else {
                (key, false, REG_NOTIFY_CHANGE_NAME)
            });
        }
        path = path.rsplit_once('\\')?.0;
        is_mods_key = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radius::RoundMaximizedAndSnapped as Round;
    use std::sync::mpsc;
    use std::time::Instant;
    use windows::Win32::System::Registry::{
        RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY_CURRENT_USER, KEY_WRITE,
        REG_OPTION_NON_VOLATILE,
    };

    /// A throwaway registry tree under HKCU that is deleted when the test ends. The real
    /// code reads HKLM, which tests must not touch; the logic is the same, only the root differs.
    struct Scratch {
        base: String,
    }

    impl Scratch {
        fn new(tag: &str) -> Scratch {
            let base = format!(r"Software\BaristaTest-{}-{tag}", std::process::id());
            Scratch { base }
        }

        fn mods(&self) -> String {
            format!(r"{}\Engine\Mods", self.base)
        }

        fn create(&self, sub: &str) {
            let path = if sub.is_empty() {
                self.base.clone()
            } else {
                format!(r"{}\{sub}", self.base)
            };
            let mut key = HKEY::default();
            // SAFETY: valid out-pointer and strings; the key is closed right below.
            unsafe {
                let status = RegCreateKeyExW(
                    HKEY_CURRENT_USER,
                    &HSTRING::from(path),
                    None,
                    PCWSTR::null(),
                    REG_OPTION_NON_VOLATILE,
                    KEY_WRITE,
                    None,
                    &mut key,
                    None,
                );
                assert_eq!(status, ERROR_SUCCESS, "create {sub}");
                let _ = RegCloseKey(key);
            }
        }

        fn set(&self, sub: &str, name: &str, kind: REG_VALUE_TYPE, data: &[u8]) {
            self.create(sub);
            let path = format!(r"{}\{sub}", self.base);
            let mut key = HKEY::default();
            // SAFETY: valid pointers and strings; the key is closed right below.
            unsafe {
                let opened = RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    &HSTRING::from(path),
                    None,
                    KEY_WRITE,
                    &mut key,
                );
                assert_eq!(opened, ERROR_SUCCESS);
                let status = RegSetValueExW(key, &HSTRING::from(name), None, kind, Some(data));
                assert_eq!(status, ERROR_SUCCESS);
                let _ = RegCloseKey(key);
            }
        }

        fn set_dword(&self, sub: &str, name: &str, value: u32) {
            self.set(sub, name, REG_DWORD, &value.to_le_bytes());
        }

        fn set_string(&self, sub: &str, name: &str, value: &str) {
            let wide: Vec<u8> = value
                .encode_utf16()
                .chain([0])
                .flat_map(|c| c.to_le_bytes())
                .collect();
            self.set(sub, name, REG_SZ, &wide);
        }

        /// Writes the three values a real Windhawk install has.
        fn install_mod(&self, disabled: u32, radius: u32, round: &str) {
            let m = format!(r"Engine\Mods\{MOD_NAME}");
            self.set_dword(&m, "Disabled", disabled);
            self.set_dword(&format!(r"{m}\Settings"), "radius", radius);
            self.set_string(&format!(r"{m}\Settings"), "roundMaximizedAndSnapped", round);
        }

        fn read(&self) -> Option<ModSettings> {
            read_settings_at(HKEY_CURRENT_USER, &self.mods())
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            // SAFETY: valid strings; deleting our own scratch tree.
            unsafe {
                let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(self.base.as_str()));
            }
        }
    }

    #[test]
    fn reads_the_three_values() {
        let s = Scratch::new("read");
        s.install_mod(0, 26, "snapped");
        let settings = s.read().expect("mod is active");
        assert_eq!(settings.radius, 26);
        assert_eq!(settings.round_maximized_and_snapped, Round::Snapped);
    }

    #[test]
    fn disabled_keep_original_and_missing_are_not_active() {
        let s = Scratch::new("inactive");
        assert!(s.read().is_none(), "nothing installed");
        s.install_mod(1, 26, "none");
        assert!(s.read().is_none(), "Disabled = 1");
        s.install_mod(0, u32::MAX, "none");
        assert!(s.read().is_none(), "radius -1 (keep original)");
        s.install_mod(0, 12, "none");
        assert_eq!(s.read().map(|m| m.radius), Some(12), "enabled again");
    }

    #[test]
    fn wrong_value_types_count_as_missing() {
        let s = Scratch::new("types");
        let m = format!(r"Engine\Mods\{MOD_NAME}");
        s.set_string(&m, "Disabled", "0"); // a string where a DWORD belongs: missing = enabled
        s.set_string(&format!(r"{m}\Settings"), "radius", "26"); // radius must be a DWORD
        assert!(s.read().is_none(), "no usable radius");
        s.set_dword(&format!(r"{m}\Settings"), "radius", 26);
        s.set_dword(&format!(r"{m}\Settings"), "roundMaximizedAndSnapped", 1); // DWORD, not a string
        let settings = s.read().expect("radius is fine");
        assert_eq!(settings.round_maximized_and_snapped, Round::Never);
    }

    /// A callback that sends a message each time it runs.
    fn counter() -> (impl Fn() + Send + 'static, mpsc::Receiver<()>) {
        let (tx, rx) = mpsc::channel();
        (
            move || {
                let _ = tx.send(());
            },
            rx,
        )
    }

    /// Waits for the next call and then drains the ones that were already queued.
    fn next_call(rx: &mpsc::Receiver<()>) {
        rx.recv_timeout(Duration::from_secs(3))
            .expect("the watcher did not call back");
        std::thread::sleep(Duration::from_millis(150));
        while rx.try_recv().is_ok() {}
    }

    #[test]
    fn watcher_reports_value_changes_instantly() {
        let s = Scratch::new("watch");
        s.install_mod(0, 26, "none");
        let (callback, rx) = counter();
        let _watcher = watch_at(HKEY_CURRENT_USER, &s.mods(), callback).unwrap();
        next_call(&rx); // the first call, right after starting

        s.set_dword(&format!(r"Engine\Mods\{MOD_NAME}\Settings"), "radius", 4);
        next_call(&rx);
        assert_eq!(s.read().map(|m| m.radius), Some(4));

        s.set_dword(&format!(r"Engine\Mods\{MOD_NAME}"), "Disabled", 1);
        next_call(&rx);
        assert!(s.read().is_none());
    }

    #[test]
    fn watcher_notices_keys_that_appear_later() {
        let s = Scratch::new("later");
        s.create(""); // only the scratch root exists: no Engine, no Mods
        let (callback, rx) = counter();
        let _watcher = watch_at(HKEY_CURRENT_USER, &s.mods(), callback).unwrap();
        next_call(&rx);

        s.create("Engine");
        next_call(&rx);
        s.create(r"Engine\Mods");
        next_call(&rx);
        s.install_mod(0, 9, "snappedAndMaximized");
        next_call(&rx);
        assert_eq!(s.read().map(|m| m.radius), Some(9));
    }

    #[test]
    fn dropping_the_watcher_stops_its_thread_quickly() {
        let s = Scratch::new("stop");
        s.install_mod(0, 26, "none");
        let (callback, rx) = counter();
        let watcher = watch_at(HKEY_CURRENT_USER, &s.mods(), callback).unwrap();
        next_call(&rx);
        let started = Instant::now();
        drop(watcher);
        assert!(started.elapsed() < Duration::from_secs(1));
        // Nothing is listening any more.
        s.set_dword(&format!(r"Engine\Mods\{MOD_NAME}\Settings"), "radius", 3);
        std::thread::sleep(Duration::from_millis(300));
        assert!(rx.try_recv().is_err());
    }
}
