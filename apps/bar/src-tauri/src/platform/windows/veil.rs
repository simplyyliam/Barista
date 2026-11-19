//! Hides the popup windows an app opens while the bar reads its menu through UI Automation, so the user
//! never sees the app's own menu flash. The windows are made fully transparent (not moved or closed) and
//! made opaque again when the veil is dropped (the layered style stays, which is harmless).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows::core::BOOL;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowLongPtrW, IsWindowVisible, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA,
    WS_EX_LAYERED,
};

fn windows_of(pid: u32) -> Vec<isize> {
    unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: lparam is the pointer to the Vec below, alive for the whole EnumWindows call.
        let found = unsafe { &mut *(lparam.0 as *mut Vec<isize>) };
        // SAFETY: plain call.
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            found.push(hwnd.0 as isize);
        }
        BOOL(1)
    }
    let mut found: Vec<isize> = Vec::new();
    // SAFETY: the callback only touches the Vec we pass.
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut found as *mut Vec<isize> as isize));
    }
    found.into_iter().filter(|h| super::app_info::pid_of(*h) == pid).collect()
}

pub struct Veil {
    stop: Arc<AtomicBool>,
    hidden: Arc<Mutex<Vec<isize>>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Veil {
    /// Starts hiding every window of `pid` that shows up from now on (the windows it has now are left alone).
    pub fn start(pid: u32) -> Veil {
        let stop = Arc::new(AtomicBool::new(false));
        let hidden = Arc::new(Mutex::new(Vec::new()));
        let known = windows_of(pid);
        let (stop2, hidden2) = (stop.clone(), hidden.clone());
        let thread = std::thread::Builder::new()
            .name("uia-veil".into())
            .spawn(move || {
                while !stop2.load(Ordering::SeqCst) {
                    for hwnd in windows_of(pid) {
                        let mut list = hidden2.lock().unwrap_or_else(|e| e.into_inner());
                        if known.contains(&hwnd) || list.contains(&hwnd) {
                            continue;
                        }
                        let window = HWND(hwnd as *mut _);
                        // SAFETY: plain calls on a window of another process; failing leaves it as it was.
                        unsafe {
                            let style = GetWindowLongPtrW(window, GWL_EXSTYLE);
                            SetWindowLongPtrW(window, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
                            let _ = SetLayeredWindowAttributes(window, COLORREF(0), 0, LWA_ALPHA);
                        }
                        list.push(hwnd);
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            })
            .ok();
        Veil { stop, hidden, thread }
    }
}

impl Drop for Veil {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        for hwnd in self.hidden.lock().unwrap_or_else(|e| e.into_inner()).drain(..) {
            let window = HWND(hwnd as *mut _);
            // SAFETY: plain calls; the window may be gone already, then they fail harmlessly.
            unsafe {
                let _ = SetLayeredWindowAttributes(window, COLORREF(0), 255, LWA_ALPHA);
            }
        }
    }
}
