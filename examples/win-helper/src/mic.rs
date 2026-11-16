//! Microphone mute: the default capture device's mute switch, the same one Windows' own mic
//! mute uses. One thread owns every COM object and sleeps in `recv()`; Windows' callbacks only
//! send it a message (calling COM from inside a callback can deadlock). No polling.

use super::Out;
use serde_json::{json, Value};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use windows::core::{implement, Result as WinResult, PCWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator,
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
};

enum Msg {
    /// The mute state may have changed: read it again.
    Refresh,
    /// The default microphone changed, appeared or went away: bind to the current one.
    Rebind,
    Command(Value),
}

#[implement(IAudioEndpointVolumeCallback)]
struct MuteCallback {
    tx: Sender<Msg>,
}

impl IAudioEndpointVolumeCallback_Impl for MuteCallback_Impl {
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
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, _role: ERole, _id: &PCWSTR) -> WinResult<()> {
        if flow == eCapture {
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
    // SAFETY: plain COM calls on live objects.
    unsafe {
        let Ok(store) = device.OpenPropertyStore(STGM_READ) else {
            return String::new();
        };
        let Ok(value) = store.GetValue(&PKEY_Device_FriendlyName) else {
            return String::new();
        };
        value.to_string()
    }
}

fn bind(enumerator: &IMMDeviceEnumerator, tx: &Sender<Msg>) -> Option<Bound> {
    // SAFETY: plain COM calls; `Bound` unregisters the callback when dropped.
    unsafe {
        let device = enumerator.GetDefaultAudioEndpoint(eCapture, eCommunications).ok()?;
        let endpoint: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
        let callback: IAudioEndpointVolumeCallback = MuteCallback { tx: tx.clone() }.into();
        endpoint.RegisterControlChangeNotify(&callback).ok()?;
        Some(Bound { endpoint, callback, name: friendly_name(&device) })
    }
}

fn report(out: &Out, bound: &Option<Bound>) {
    let event = match bound {
        Some(b) => {
            // SAFETY: plain COM call on a live endpoint.
            let muted = unsafe { b.endpoint.GetMute() }.map(|m| m.as_bool()).unwrap_or(false);
            json!({"type": "mic", "present": true, "muted": muted, "device": b.name})
        }
        None => json!({"type": "mic", "present": false, "muted": false, "device": ""}),
    };
    out.emit(event);
}

/// Starts the worker thread and returns where commands go.
pub fn start(out: Arc<Out>) -> Sender<Value> {
    let (cmd_tx, cmd_rx) = channel::<Value>();
    let (tx, rx) = channel::<Msg>();
    // Commands from stdin join the same queue as Windows' events.
    let forward = tx.clone();
    std::thread::spawn(move || {
        while let Ok(v) = cmd_rx.recv() {
            if forward.send(Msg::Command(v)).is_err() {
                break;
            }
        }
        // stdin closed: the parent is gone.
        std::process::exit(0);
    });
    std::thread::spawn(move || {
        // SAFETY: first COM call on this thread.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        // SAFETY: plain COM creation.
        let Ok(enumerator) = (unsafe {
            CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL)
        }) else {
            eprintln!("win-helper: audio not available");
            out.emit(json!({"type": "mic", "present": false, "muted": false, "device": ""}));
            return;
        };
        let watcher: IMMNotificationClient = DeviceWatcher { tx: tx.clone() }.into();
        // SAFETY: `watcher` stays alive in this function until the thread ends.
        if let Err(e) = unsafe { enumerator.RegisterEndpointNotificationCallback(&watcher) } {
            eprintln!("win-helper: microphone changes not watched: {e}");
        }
        let mut bound = bind(&enumerator, &tx);
        report(&out, &bound);
        while let Ok(msg) = rx.recv() {
            match msg {
                Msg::Rebind => {
                    // Drop first so the old callback is unregistered before the new one is added.
                    drop(bound.take());
                    bound = bind(&enumerator, &tx);
                }
                Msg::Refresh => {}
                Msg::Command(v) => {
                    if let Some(b) = &bound {
                        // SAFETY: plain COM calls on a live endpoint; the null context means no
                        // event source we track.
                        unsafe {
                            let now = b.endpoint.GetMute().map(|m| m.as_bool()).unwrap_or(false);
                            let want = match v.get("cmd").and_then(Value::as_str) {
                                Some("toggle") => Some(!now),
                                Some("setMuted") => v.get("muted").and_then(Value::as_bool),
                                _ => None,
                            };
                            if let Some(want) = want {
                                let _ = b.endpoint.SetMute(want, std::ptr::null());
                            }
                        }
                    }
                }
            }
            report(&out, &bound);
        }
    });
    cmd_tx
}
