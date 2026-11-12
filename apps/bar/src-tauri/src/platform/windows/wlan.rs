//! Wi-Fi through the WLAN API: the networks in range, which one we are on, scan, connect to a
//! saved network, disconnect.
//!
//! One thread owns the WLAN handle. `WlanRegisterNotification` calls us on a Windows thread, where
//! calling the WLAN API again can deadlock, so the callback only sends a message to the owner
//! thread, which re-reads everything. Nothing polls.

use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_SUCCESS, HANDLE};
use windows::Win32::NetworkManagement::WiFi::{
    dot11_BSS_type_any, wlan_connection_mode_profile, WlanCloseHandle, WlanConnect, WlanDisconnect,
    WlanEnumInterfaces, WlanFreeMemory, WlanGetAvailableNetworkList, WlanOpenHandle,
    WlanRegisterNotification, WlanScan, L2_NOTIFICATION_DATA, WLAN_AVAILABLE_NETWORK_LIST,
    WLAN_CONNECTION_PARAMETERS, WLAN_INTERFACE_INFO_LIST, WLAN_NOTIFICATION_SOURCE_ACM,
};

/// `WLAN_AVAILABLE_NETWORK_CONNECTED` and `..._HAS_PROFILE` from `dwFlags`.
const FLAG_CONNECTED: u32 = 1;
const FLAG_HAS_PROFILE: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawNetwork {
    pub ssid: String,
    /// 0 to 100.
    pub signal: u8,
    pub secured: bool,
    pub connected: bool,
    /// The saved profile name; `Some` means Windows knows the password (or it is open and saved).
    pub profile: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RawWifi {
    pub has_interface: bool,
    /// Windows hides network names from apps without location access (24H2 and later).
    pub location_denied: bool,
    pub networks: Vec<RawNetwork>,
}

enum Msg {
    Refresh,
    Scan,
    Connect(String),
    Disconnect,
}

static TX: OnceLock<Sender<Msg>> = OnceLock::new();

pub fn scan() {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::Scan);
    }
}

/// Connects to a saved profile by name.
pub fn connect(profile: String) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::Connect(profile));
    }
}

pub fn disconnect() {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::Disconnect);
    }
}

/// Runs on a Windows thread: only tells the owner thread to look again.
unsafe extern "system" fn on_notification(_data: *mut L2_NOTIFICATION_DATA, _ctx: *mut core::ffi::c_void) {
    if let Some(tx) = TX.get() {
        let _ = tx.send(Msg::Refresh);
    }
}

fn utf16_until_nul(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// The first Wi-Fi adapter. Most laptops have one.
fn first_interface(handle: HANDLE) -> Option<GUID> {
    let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
    // SAFETY: Windows fills `list`; it is freed below. The slice stays inside what Windows allocated.
    unsafe {
        if WlanEnumInterfaces(handle, None, &mut list) != ERROR_SUCCESS.0 || list.is_null() {
            return None;
        }
        let items = std::slice::from_raw_parts(
            (*list).InterfaceInfo.as_ptr(),
            (*list).dwNumberOfItems as usize,
        );
        let guid = items.first().map(|i| i.InterfaceGuid);
        WlanFreeMemory(list.cast());
        guid
    }
}

fn read(handle: HANDLE) -> RawWifi {
    let Some(guid) = first_interface(handle) else {
        return RawWifi::default();
    };
    let mut out = RawWifi {
        has_interface: true,
        ..RawWifi::default()
    };
    let mut list: *mut WLAN_AVAILABLE_NETWORK_LIST = std::ptr::null_mut();
    // SAFETY: as above; the pointer is freed before returning.
    unsafe {
        let status = WlanGetAvailableNetworkList(handle, &guid, 0, None, &mut list);
        if status == ERROR_ACCESS_DENIED.0 {
            out.location_denied = true;
            return out;
        }
        if status != ERROR_SUCCESS.0 || list.is_null() {
            return out;
        }
        let items = std::slice::from_raw_parts(
            (*list).Network.as_ptr(),
            (*list).dwNumberOfItems as usize,
        );
        for n in items {
            let len = (n.dot11Ssid.uSSIDLength as usize).min(n.dot11Ssid.ucSSID.len());
            let ssid = String::from_utf8_lossy(&n.dot11Ssid.ucSSID[..len]).into_owned();
            out.networks.push(RawNetwork {
                ssid,
                signal: n.wlanSignalQuality.min(100) as u8,
                secured: n.bSecurityEnabled.as_bool(),
                connected: n.dwFlags & FLAG_CONNECTED != 0,
                profile: (n.dwFlags & FLAG_HAS_PROFILE != 0).then(|| utf16_until_nul(&n.strProfileName)),
            });
        }
        WlanFreeMemory(list.cast());
    }
    out
}

pub fn start(on_change: fn(RawWifi)) {
    let (tx, rx) = channel();
    if TX.set(tx).is_err() {
        return;
    }
    let spawned = std::thread::Builder::new()
        .name("barista-wlan".into())
        .spawn(move || {
            let mut handle = HANDLE::default();
            let mut negotiated = 0u32;
            // SAFETY: plain call; the handle is closed when the thread ends.
            let opened = unsafe { WlanOpenHandle(2, None, &mut negotiated, &mut handle) };
            if opened != ERROR_SUCCESS.0 {
                // No WLAN service (a desktop without Wi-Fi): report "no Wi-Fi" and stop.
                on_change(RawWifi::default());
                return;
            }
            // SAFETY: the callback is a plain function and needs no context.
            unsafe {
                WlanRegisterNotification(
                    handle,
                    WLAN_NOTIFICATION_SOURCE_ACM,
                    false,
                    Some(on_notification),
                    None,
                    None,
                    None,
                );
            }
            on_change(read(handle));
            while let Ok(first) = rx.recv() {
                let mut jobs = vec![first];
                while let Ok(more) = rx.try_recv() {
                    jobs.push(more);
                }
                for job in jobs {
                    let Some(guid) = first_interface(handle) else {
                        continue;
                    };
                    // SAFETY: `guid` and the parameter struct live for the call.
                    unsafe {
                        match job {
                            Msg::Refresh => {}
                            Msg::Scan => {
                                let _ = WlanScan(handle, &guid, None, None, None);
                            }
                            Msg::Disconnect => {
                                let _ = WlanDisconnect(handle, &guid, None);
                            }
                            Msg::Connect(profile) => {
                                let mut name: Vec<u16> = profile.encode_utf16().collect();
                                name.push(0);
                                let params = WLAN_CONNECTION_PARAMETERS {
                                    wlanConnectionMode: wlan_connection_mode_profile,
                                    strProfile: PCWSTR(name.as_ptr()),
                                    pDot11Ssid: std::ptr::null_mut(),
                                    pDesiredBssidList: std::ptr::null_mut(),
                                    dot11BssType: dot11_BSS_type_any,
                                    dwFlags: 0,
                                };
                                let _ = WlanConnect(handle, &guid, &params, None);
                            }
                        }
                    }
                }
                on_change(read(handle));
            }
            // SAFETY: closes the handle opened above.
            unsafe {
                WlanCloseHandle(handle, None);
            }
        });
    if let Err(e) = spawned {
        eprintln!("barista: could not start the wlan thread: {e}");
    }
}
