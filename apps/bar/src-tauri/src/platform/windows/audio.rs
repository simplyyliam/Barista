//! Core Audio for the default output device: level, mute and change notifications.
//!
//! One thread owns every COM object and sleeps in `recv()`. Windows calls our callbacks on its own
//! threads, so they never touch COM: they only send a message to the owner thread, which re-reads
//! everything (calling COM from inside a callback can deadlock). No polling.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;
use windows::core::{implement, Result as WinResult, PCWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator, IMMNotificationClient,
    IMMNotificationClient_Impl, MMDeviceEnumerator, AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE,
    DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
};
use windows::Win32::Foundation::PROPERTYKEY;

/// What the output device says right now.
#[derive(Clone, Debug)]
pub struct RawVolume {
    pub level: f32,
    pub muted: bool,
    pub device_name: String,
}

/// One active output device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawOutput {
    pub id: String,
    pub name: String,
    pub default: bool,
}

/// The active output devices, and whether we can switch between them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RawOutputs {
    pub devices: Vec<RawOutput>,
    pub can_switch: bool,
}

enum Msg {
    /// Make this output the default (the id must be in the current list).
    SetDefault(String),
    /// Re-read level and mute (the volume callback fired).
    Refresh,
    /// The default device changed or went away: bind to the current one.
    Rebind,
    SetVolume(f32),
    SetMuted(bool),
}

static TX: OnceLock<Sender<Msg>> = OnceLock::new();

pub fn set_default_output(id: String) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::SetDefault(id));
    }
}

pub fn set_volume(level: f32) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::SetVolume(level));
    }
}

pub fn set_muted(muted: bool) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::SetMuted(muted));
    }
}

#[implement(IAudioEndpointVolumeCallback)]
struct VolumeCallback {
    tx: Sender<Msg>,
}

impl IAudioEndpointVolumeCallback_Impl for VolumeCallback_Impl {
    fn OnNotify(&self, _data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> WinResult<()> {
        let _ = self.tx.send(Msg::Refresh);
        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct DeviceWatcher {
    tx: Sender<Msg>,
}

impl IMMNotificationClient_Impl for DeviceWatcher_Impl {
    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _state: DEVICE_STATE) -> WinResult<()> {
        let _ = self.tx.send(Msg::Rebind);
        Ok(())
    }
    fn OnDeviceAdded(&self, _id: &PCWSTR) -> WinResult<()> {
        let _ = self.tx.send(Msg::Rebind);
        Ok(())
    }
    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> WinResult<()> {
        let _ = self.tx.send(Msg::Rebind);
        Ok(())
    }
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _id: &PCWSTR) -> WinResult<()> {
        if flow == eRender && role == eMultimedia {
            let _ = self.tx.send(Msg::Rebind);
        }
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> WinResult<()> {
        Ok(())
    }
}

/// The endpoint we are bound to, with its callback so it can be unregistered.
struct Bound {
    endpoint: IAudioEndpointVolume,
    callback: IAudioEndpointVolumeCallback,
    name: String,
}

impl Drop for Bound {
    fn drop(&mut self) {
        // SAFETY: the callback was registered on this endpoint.
        unsafe {
            let _ = self.endpoint.UnregisterControlChangeNotify(&self.callback);
        }
    }
}

fn friendly_name(device: &IMMDevice) -> String {
    // SAFETY: plain COM calls on live objects; the string is freed after copying.
    unsafe {
        let Ok(store) = device.OpenPropertyStore(STGM_READ) else {
            return String::new();
        };
        let Ok(value) = store.GetValue(&PKEY_Device_FriendlyName) else {
            return String::new();
        };
        // PROPVARIANT frees itself when dropped and prints a string value as plain text.
        value.to_string()
    }
}

fn device_id(device: &IMMDevice) -> Option<String> {
    // SAFETY: Windows allocates the string; it is copied and freed right away.
    unsafe {
        let raw = device.GetId().ok()?;
        let id = raw.to_string().ok();
        CoTaskMemFree(Some(raw.0.cast_const().cast()));
        id
    }
}

fn list_outputs(enumerator: &IMMDeviceEnumerator, can_switch: bool) -> RawOutputs {
    // SAFETY: plain COM calls on live objects.
    unsafe {
        let default = enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .ok()
            .and_then(|d| device_id(&d));
        let Ok(collection) = enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) else {
            return RawOutputs::default();
        };
        let count = collection.GetCount().unwrap_or(0);
        let devices = (0..count)
            .filter_map(|i| collection.Item(i).ok())
            .filter_map(|d| {
                let id = device_id(&d)?;
                Some(RawOutput {
                    default: default.as_deref() == Some(id.as_str()),
                    name: friendly_name(&d),
                    id,
                })
            })
            .collect();
        RawOutputs { devices, can_switch }
    }
}

fn bind(enumerator: &IMMDeviceEnumerator, tx: &Sender<Msg>) -> Option<Bound> {
    // SAFETY: plain COM calls; `Bound` unregisters the callback when dropped.
    unsafe {
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia).ok()?;
        let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
        let callback: IAudioEndpointVolumeCallback = VolumeCallback { tx: tx.clone() }.into();
        endpoint.RegisterControlChangeNotify(&callback).ok()?;
        Some(Bound {
            endpoint,
            callback,
            name: friendly_name(&device),
        })
    }
}

fn read(bound: &Bound) -> Option<RawVolume> {
    // SAFETY: plain COM calls on a live endpoint.
    unsafe {
        Some(RawVolume {
            level: bound.endpoint.GetMasterVolumeLevelScalar().ok()?,
            muted: bound.endpoint.GetMute().ok()?.as_bool(),
            device_name: bound.name.clone(),
        })
    }
}

fn run(
    rx: Receiver<Msg>,
    tx: Sender<Msg>,
    on_change: fn(Option<RawVolume>),
    on_outputs: fn(RawOutputs),
) {
    // SAFETY: first COM call on this thread; it keeps the apartment for its whole life.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    // SAFETY: plain COM creation.
    let Ok(enumerator) = (unsafe {
        CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
    }) else {
        eprintln!("barista: audio not available");
        on_change(None);
        on_outputs(RawOutputs::default());
        return;
    };
    let watcher: IMMNotificationClient = DeviceWatcher { tx: tx.clone() }.into();
    // SAFETY: `watcher` stays alive in this function until the thread ends.
    if let Err(e) = unsafe { enumerator.RegisterEndpointNotificationCallback(&watcher) } {
        eprintln!("barista: audio device changes not watched: {e}");
    }
    // Created on this thread, once. `None` = Windows no longer has the interface: the UI opens the
    // sound settings page instead of switching.
    let policy = super::audio_policy::create();
    let can_switch = policy.is_some();
    let mut bound = bind(&enumerator, &tx);
    on_change(bound.as_ref().and_then(read));
    on_outputs(list_outputs(&enumerator, can_switch));
    while let Ok(msg) = rx.recv() {
        let relist = matches!(msg, Msg::Rebind | Msg::SetDefault(_));
        match msg {
            Msg::Rebind => {
                // Drop first so the old callback is unregistered before the new one is added.
                drop(bound.take());
                bound = bind(&enumerator, &tx);
            }
            Msg::SetVolume(level) => {
                if let Some(b) = &bound {
                    // SAFETY: a plain call; the null context means no event source we track.
                    let _ = unsafe {
                        b.endpoint
                            .SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null())
                    };
                }
            }
            Msg::SetMuted(muted) => {
                if let Some(b) = &bound {
                    // SAFETY: as above.
                    let _ = unsafe { b.endpoint.SetMute(muted, std::ptr::null()) };
                }
            }
            Msg::SetDefault(id) => {
                // Only ids Windows just listed go to COM, never a string from the UI as it is.
                let known = list_outputs(&enumerator, can_switch)
                    .devices
                    .iter()
                    .any(|d| d.id == id);
                let switched = known && policy.as_ref().is_some_and(|p| p.set_default(&id));
                if !switched {
                    super::shell::open_uri("ms-settings:sound");
                }
            }
            Msg::Refresh => {}
        }
        on_change(bound.as_ref().and_then(read));
        // A volume tick must not re-list devices (property store reads): only device changes do.
        if relist {
            on_outputs(list_outputs(&enumerator, can_switch));
        }
    }
}

/// Starts the audio thread. `on_change` runs on it with the full state (`None` = no output device).
pub fn start(on_change: fn(Option<RawVolume>), on_outputs: fn(RawOutputs)) {
    let (tx, rx) = channel();
    if TX.set(tx.clone()).is_err() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("barista-audio".into())
        .spawn(move || run(rx, tx, on_change, on_outputs));
    if let Err(e) = spawned {
        eprintln!("barista: could not start the audio thread: {e}");
    }
}
