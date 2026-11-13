//! Per-app volume: the audio sessions on the default output device (what Windows' volume mixer
//! shows). Lives on the audio thread (`audio.rs`) and follows its rules: callbacks from Windows
//! never touch COM, they only tell the audio thread what to re-read (no polling), and an app id
//! from the UI is only used if it is in the list we read ourselves.
//!
//! One app can have many sessions (a browser has one per tab group), so sessions are grouped by
//! program file: one slider moves all of them.

use base64::{engine::general_purpose::STANDARD, Engine};
use std::collections::HashMap;
use std::sync::Arc;
use windows::core::{implement, Interface, Result as WinResult, BOOL, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, AudioSessionDisconnectReason, AudioSessionState, AudioSessionStateActive,
    AudioSessionStateExpired, IAudioSessionControl, IAudioSessionControl2, IAudioSessionEvents,
    IAudioSessionEvents_Impl, IAudioSessionManager2, IAudioSessionNotification,
    IAudioSessionNotification_Impl, IMMDeviceEnumerator, ISimpleAudioVolume,
};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};

/// What a Windows callback asks the audio thread to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// A session came, went or changed state: list the sessions again.
    Sessions,
    /// Only a volume or mute changed: read the levels again.
    Levels,
}

/// Called on Windows' threads: it must only send a message.
pub type Notify = Arc<dyn Fn(Change) + Send + Sync>;

/// One audio session, before grouping.
#[derive(Clone, Debug, PartialEq)]
pub struct RawSession {
    pub key: String,
    pub name: String,
    pub icon: Option<String>,
    pub level: f32,
    pub muted: bool,
    pub playing: bool,
}

/// One app (all its sessions together).
#[derive(Clone, Debug, PartialEq)]
pub struct RawApp {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub level: f32,
    pub muted: bool,
    /// At least one of its sessions is making sound right now.
    pub playing: bool,
}

/// Groups sessions by app: the loudest session sets the level, the app is muted when every
/// session is. Playing apps first, then by name.
pub fn group(sessions: Vec<RawSession>) -> Vec<RawApp> {
    let mut apps: Vec<RawApp> = Vec::new();
    for s in sessions {
        match apps.iter_mut().find(|a| a.id == s.key) {
            Some(app) => {
                app.level = app.level.max(s.level);
                app.muted &= s.muted;
                app.playing |= s.playing;
                if app.icon.is_none() {
                    app.icon = s.icon;
                }
            }
            None => apps.push(RawApp {
                id: s.key,
                name: s.name,
                icon: s.icon,
                level: s.level,
                muted: s.muted,
                playing: s.playing,
            }),
        }
    }
    apps.sort_by(|a, b| {
        b.playing
            .cmp(&a.playing)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });
    apps
}

#[implement(IAudioSessionNotification)]
struct Created {
    notify: Notify,
}

impl IAudioSessionNotification_Impl for Created_Impl {
    fn OnSessionCreated(
        &self,
        _new_session: windows_core::Ref<'_, IAudioSessionControl>,
    ) -> WinResult<()> {
        (self.notify)(Change::Sessions);
        Ok(())
    }
}

#[implement(IAudioSessionEvents)]
struct Events {
    notify: Notify,
}

impl IAudioSessionEvents_Impl for Events_Impl {
    fn OnDisplayNameChanged(&self, _name: &PCWSTR, _ctx: *const windows::core::GUID) -> WinResult<()> {
        Ok(())
    }
    fn OnIconPathChanged(&self, _path: &PCWSTR, _ctx: *const windows::core::GUID) -> WinResult<()> {
        Ok(())
    }
    fn OnSimpleVolumeChanged(
        &self,
        _volume: f32,
        _muted: BOOL,
        _ctx: *const windows::core::GUID,
    ) -> WinResult<()> {
        (self.notify)(Change::Levels);
        Ok(())
    }
    fn OnChannelVolumeChanged(
        &self,
        _count: u32,
        _volumes: *const f32,
        _changed: u32,
        _ctx: *const windows::core::GUID,
    ) -> WinResult<()> {
        Ok(())
    }
    fn OnGroupingParamChanged(
        &self,
        _param: *const windows::core::GUID,
        _ctx: *const windows::core::GUID,
    ) -> WinResult<()> {
        Ok(())
    }
    fn OnStateChanged(&self, _state: AudioSessionState) -> WinResult<()> {
        (self.notify)(Change::Sessions);
        Ok(())
    }
    fn OnSessionDisconnected(&self, _reason: AudioSessionDisconnectReason) -> WinResult<()> {
        (self.notify)(Change::Sessions);
        Ok(())
    }
}

/// A live session with its volume control and the registration that tells us about changes.
struct Entry {
    key: String,
    name: String,
    icon: Option<String>,
    playing: bool,
    simple: ISimpleAudioVolume,
    control: IAudioSessionControl,
    events: IAudioSessionEvents,
}

impl Drop for Entry {
    fn drop(&mut self) {
        // SAFETY: `events` was registered on this control when the entry was made.
        unsafe {
            let _ = self.control.UnregisterAudioSessionNotification(&self.events);
        }
    }
}

/// The sessions of the default output device.
pub struct Sessions {
    manager: IAudioSessionManager2,
    created: IAudioSessionNotification,
    notify: Notify,
    entries: Vec<Entry>,
    /// Icon per program file, so each one is drawn once.
    icons: HashMap<String, Option<String>>,
}

impl Drop for Sessions {
    fn drop(&mut self) {
        self.entries.clear();
        // SAFETY: `created` was registered on this manager in `bind`.
        unsafe {
            let _ = self.manager.UnregisterSessionNotification(&self.created);
        }
    }
}

impl Sessions {
    /// Binds to the default output device. `None` when there is no device or no session manager.
    pub fn bind(enumerator: &IMMDeviceEnumerator, notify: Notify) -> Option<Self> {
        // SAFETY: plain COM calls; `Drop` unregisters the notification.
        unsafe {
            let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia).ok()?;
            let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None).ok()?;
            let created: IAudioSessionNotification = Created { notify: notify.clone() }.into();
            manager.RegisterSessionNotification(&created).ok()?;
            Some(Self {
                manager,
                created,
                notify,
                entries: Vec::new(),
                icons: HashMap::new(),
            })
        }
    }

    /// Lists the sessions again, re-registers for their changes and returns the grouped apps.
    pub fn rebuild(&mut self) -> Vec<RawApp> {
        // Unregister the old ones first (Drop), so no change is reported twice.
        self.entries.clear();
        // SAFETY: plain COM calls on live objects.
        let found = unsafe { self.enumerate() };
        self.entries = found;
        self.levels()
    }

    /// Reads the current levels of the sessions we already know.
    pub fn levels(&self) -> Vec<RawApp> {
        group(
            self.entries
                .iter()
                .map(|e| RawSession {
                    key: e.key.clone(),
                    name: e.name.clone(),
                    icon: e.icon.clone(),
                    // SAFETY: plain COM calls on live objects.
                    level: unsafe { e.simple.GetMasterVolume().unwrap_or(0.0) },
                    muted: unsafe { e.simple.GetMute().map(|m| m.as_bool()).unwrap_or(false) },
                    playing: e.playing,
                })
                .collect(),
        )
    }

    /// Sets the volume of every session of an app. An id that is not in our list does nothing.
    pub fn set_volume(&self, id: &str, level: f32) {
        for e in self.entries.iter().filter(|e| e.key == id) {
            // SAFETY: plain call; the null context means no event source we track.
            let _ = unsafe { e.simple.SetMasterVolume(level.clamp(0.0, 1.0), std::ptr::null()) };
        }
    }

    pub fn set_muted(&self, id: &str, muted: bool) {
        for e in self.entries.iter().filter(|e| e.key == id) {
            // SAFETY: as above.
            let _ = unsafe { e.simple.SetMute(muted, std::ptr::null()) };
        }
    }

    unsafe fn enumerate(&mut self) -> Vec<Entry> {
        let Ok(list) = self.manager.GetSessionEnumerator() else {
            return Vec::new();
        };
        let own = GetCurrentProcessId();
        let count = list.GetCount().unwrap_or(0);
        let mut out = Vec::new();
        for i in 0..count {
            let Ok(control) = list.GetSession(i) else { continue };
            let Ok(state) = control.GetState() else { continue };
            if state == AudioSessionStateExpired {
                continue;
            }
            let Ok(control2) = control.cast::<IAudioSessionControl2>() else { continue };
            // S_OK means "this is the system sounds session": not an app.
            if control2.IsSystemSoundsSession().0 == 0 {
                continue;
            }
            let pid = control2.GetProcessId().unwrap_or(0);
            if pid == 0 || pid == own {
                continue;
            }
            let Ok(simple) = control.cast::<ISimpleAudioVolume>() else { continue };
            let (key, name, icon) = self.identify(pid);
            let events: IAudioSessionEvents = Events { notify: self.notify.clone() }.into();
            if control.RegisterAudioSessionNotification(&events).is_err() {
                continue;
            }
            out.push(Entry {
                key,
                name,
                icon,
                playing: state == AudioSessionStateActive,
                simple,
                control,
                events,
            });
        }
        out
    }

    /// Key, name and icon of the program behind a process id.
    unsafe fn identify(&mut self, pid: u32) -> (String, String, Option<String>) {
        let Some(path) = exe_path(pid) else {
            // Protected process: we can't read its file, so it can only be told apart by pid.
            return (format!("pid:{pid}"), "Unknown app".into(), None);
        };
        let file = path.rsplit('\\').next().unwrap_or(&path).to_string();
        let key = file.to_lowercase();
        let name = file_description(&path)
            .filter(|d| !d.trim().is_empty())
            .unwrap_or_else(|| title_from_file(&file));
        let icon = self
            .icons
            .entry(key.clone())
            .or_insert_with(|| {
                super::app_icon::icon_png_for_file(&path)
                    .map(|png| format!("data:image/png;base64,{}", STANDARD.encode(png)))
            })
            .clone();
        (key, name, icon)
    }
}

/// "spotify.exe" -> "Spotify" (used when the file has no description).
fn title_from_file(file: &str) -> String {
    let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
    let mut chars = stem.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_else(|| "App".into())
}

fn exe_path(pid: u32) -> Option<String> {
    // SAFETY: the handle is ours and closed on every path; the buffer outlives the call.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(handle);
        ok.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// The "FileDescription" of a program file ("Google Chrome"), from its version resource.
fn file_description(path: &str) -> Option<String> {
    // SAFETY: the buffer outlives every call that points into it; pointers returned by
    // `VerQueryValueW` point inside that buffer and are only read before it is dropped.
    unsafe {
        let wide = HSTRING::from(path);
        let size = GetFileVersionInfoSizeW(&wide, None);
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(&wide, None, size, data.as_mut_ptr().cast()).ok()?;

        let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let translation = HSTRING::from("\\VarFileInfo\\Translation");
        if !VerQueryValueW(data.as_ptr().cast(), &translation, &mut ptr, &mut len).as_bool() || len < 4 {
            return None;
        }
        let pair = std::slice::from_raw_parts(ptr as *const u16, 2);
        let query = HSTRING::from(format!(
            "\\StringFileInfo\\{:04x}{:04x}\\FileDescription",
            pair[0], pair[1]
        ));
        if !VerQueryValueW(data.as_ptr().cast(), &query, &mut ptr, &mut len).as_bool() || len == 0 {
            return None;
        }
        PCWSTR(ptr as *const u16).to_string().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(key: &str, name: &str, level: f32, muted: bool, playing: bool) -> RawSession {
        RawSession {
            key: key.into(),
            name: name.into(),
            icon: None,
            level,
            muted,
            playing,
        }
    }

    #[test]
    fn sessions_of_one_app_become_one_slider() {
        let apps = group(vec![
            s("chrome.exe", "Chrome", 0.2, true, false),
            s("chrome.exe", "Chrome", 0.8, false, true),
        ]);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].level, 0.8, "the loudest session sets the level");
        assert!(!apps[0].muted, "muted only when every session is");
        assert!(apps[0].playing);
    }

    #[test]
    fn app_is_muted_when_all_its_sessions_are() {
        let apps = group(vec![s("a.exe", "A", 0.5, true, false), s("a.exe", "A", 0.5, true, false)]);
        assert!(apps[0].muted);
    }

    #[test]
    fn playing_apps_come_first_then_by_name() {
        let apps = group(vec![
            s("z.exe", "Zoom", 1.0, false, false),
            s("b.exe", "browser", 1.0, false, true),
            s("a.exe", "Apple Music", 1.0, false, true),
            s("m.exe", "Mail", 1.0, false, false),
        ]);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Apple Music", "browser", "Mail", "Zoom"]);
    }

    #[test]
    fn title_comes_from_the_file_name() {
        assert_eq!(title_from_file("spotify.exe"), "Spotify");
        assert_eq!(title_from_file("msedge"), "Msedge");
        assert_eq!(title_from_file(""), "App");
    }
}
