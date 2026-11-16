//! Our own hidden `Shell_TrayWnd` (ADR 0012, 0026): the window apps find when they call
//! `Shell_NotifyIcon`. We see every tray message first, copy what we need, and hand the same
//! message to Explorer's real taskbar so its tray, balloons and our own AppBar keep working.
//!
//! No injection and no memory reads: this is a normal window receiving normal messages.
//!
//! Why the order matters: `Shell_NotifyIcon` does `FindWindow("Shell_TrayWnd")`, which returns the
//! first match in z-order. Both windows are topmost, so whoever was raised last wins. Explorer
//! raises its taskbar now and then (Start, clicks on the taskbar), so we re-raise ours when an
//! event that could have changed the order fires (no timer, rule 9). `stats()` counts how often.

use crate::tray::parse::{self, Action, TrayMessage, COPYDATA_TRAY};
use std::collections::HashSet;
use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;
use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows::Win32::UI::WindowsAndMessaging::{
    ChangeWindowMessageFilterEx, CopyIcon, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyWindow, DispatchMessageW, FindWindowExW, FindWindowW, GetMessageW, GetWindowRect,
    GetWindowThreadProcessId, InSendMessage, IsWindow, PostMessageW, PostQuitMessage,
    RegisterClassW, RegisterWindowMessageW, UnregisterClassW, SendMessageTimeoutW, SetWindowPos, TranslateMessage,
    EVENT_OBJECT_DESTROY, EVENT_OBJECT_REORDER, EVENT_SYSTEM_FOREGROUND, HICON, HWND_TOPMOST,
    MSG, MSGFLT_ALLOW, OBJID_WINDOW, SMTO_ABORTIFHUNG, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, WINEVENT_OUTOFCONTEXT, WM_CLOSE, WM_COMMAND, WM_COPYDATA, WM_DESTROY,
    WM_DISPLAYCHANGE, WM_DPICHANGED, WM_SETTINGCHANGE, WM_USER, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const CLASS: PCWSTR = w!("Shell_TrayWnd");
/// How long we wait for Explorer's answer before giving up (a hung Explorer must not hang apps).
const FORWARD_TIMEOUT_MS: u32 = 1000;
/// Tray payloads are about 1 KB; refuse to copy more than this from a message.
const MAX_PAYLOAD: usize = 4096;

/// What the host tells the tray worker. The host never waits for the worker (rule 3).
pub enum HostEvent {
    /// A parsed tray message, with a private copy of its icon (the app may free its own right
    /// after the call returns) and the process that owns the window.
    Tray {
        message: TrayMessage,
        icon: Option<OwnedIcon>,
        pid: u32,
    },
    /// A window that owns tray icons was destroyed (the app quit or crashed).
    WindowGone(u32),
}

/// An icon we copied; destroyed when dropped.
pub struct OwnedIcon(HICON);

// SAFETY: an HICON is a number Windows looks up in its own table, not a pointer into our memory,
// so it can be used (and destroyed) from any thread.
unsafe impl Send for OwnedIcon {}

impl OwnedIcon {
    pub fn handle(&self) -> HICON {
        self.0
    }
}

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        // SAFETY: we own this copy and drop it exactly once.
        unsafe {
            let _ = DestroyIcon(self.0);
        }
    }
}

static OURS: AtomicIsize = AtomicIsize::new(0);
static EXPLORER: AtomicIsize = AtomicIsize::new(0);
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
/// Where host events go. Emptied when the host stops, so it can start again (ADR 0032).
static SINK: Mutex<Option<SyncSender<HostEvent>>> = Mutex::new(None);
/// Windows (as 32-bit handles) that own at least one icon: only their destruction is reported.
static WATCHED: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();

static RERAISED: AtomicU64 = AtomicU64::new(0);
static TRAY_MESSAGES: AtomicU64 = AtomicU64::new(0);
static DROPPED: AtomicU64 = AtomicU64::new(0);
static FORWARD_FAILED: AtomicU64 = AtomicU64::new(0);

/// Counters for the z-order measurement and the perf notes (`docs/perf/0.6.md`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    /// Times Explorer's window was found above ours and we raised ours again.
    pub reraised: u64,
    pub tray_messages: u64,
    /// Events the worker was too slow to take.
    pub dropped: u64,
    /// Messages Explorer did not answer in time (or at all).
    pub forward_failed: u64,
}

/// Tells every program the taskbar was (re)created, so they add their icons again. Sent once when the
/// host starts, and again by the worker if nothing has arrived a few seconds later. A hung window costs
/// at most half a second.
pub fn announce() {
    let message = TASKBAR_CREATED.load(Ordering::Relaxed);
    if message == 0 {
        return;
    }
    let mut sent = 0usize;
    // SAFETY: plain call; the result is only the count we pass a pointer to.
    let _ = unsafe {
        SendMessageTimeoutW(
            windows::Win32::UI::WindowsAndMessaging::HWND_BROADCAST,
            message,
            WPARAM(0),
            LPARAM(0),
            SMTO_ABORTIFHUNG,
            500,
            Some(&mut sent),
        )
    };
}

pub fn stats() -> Stats {
    Stats {
        reraised: RERAISED.load(Ordering::Relaxed),
        tray_messages: TRAY_MESSAGES.load(Ordering::Relaxed),
        dropped: DROPPED.load(Ordering::Relaxed),
        forward_failed: FORWARD_FAILED.load(Ordering::Relaxed),
    }
}

fn watched() -> &'static Mutex<HashSet<u32>> {
    WATCHED.get_or_init(|| Mutex::new(HashSet::new()))
}

/// A window handle as the 32-bit number tray messages carry.
fn to_32(hwnd: HWND) -> u32 {
    hwnd.0 as isize as u32
}

/// The other way round: sign-extend, like Windows does for 32-bit handles.
pub fn from_32(handle: u32) -> HWND {
    HWND(handle as i32 as isize as *mut c_void)
}

fn hwnd_of(value: isize) -> HWND {
    HWND(value as *mut c_void)
}

/// Stop reporting this window (its last icon is gone).
pub fn unwatch(hwnd: u32) {
    watched().lock().unwrap_or_else(|e| e.into_inner()).remove(&hwnd);
}

/// Is this window (a 32-bit handle from a tray message) still there?
pub fn window_exists(hwnd: u32) -> bool {
    // SAFETY: IsWindow accepts any value.
    unsafe { IsWindow(Some(from_32(hwnd))) }.as_bool()
}

/// The process that owns a window, or 0 if the window is gone.
pub fn window_pid(hwnd: u32) -> u32 {
    let mut pid = 0u32;
    // SAFETY: only writes the pid we pass a pointer to; a stale handle just gives 0.
    unsafe { GetWindowThreadProcessId(from_32(hwnd), Some(&mut pid)) };
    pid
}

/// The program name of a process (`Discord` for `...\Discord.exe`).
pub fn process_name(pid: u32) -> Option<String> {
    // SAFETY: the handle is closed before returning; the buffer outlives the call.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = [0u16; 520];
        let mut len = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        result.ok()?;
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        let file = path.rsplit(['\\', '/']).next()?;
        Some(file.strip_suffix(".exe").unwrap_or(file).to_string())
    }
}

/// Every window of class `Shell_TrayWnd`, top of the z-order first.
fn tray_windows() -> Vec<HWND> {
    let mut found = Vec::new();
    let mut after: Option<HWND> = None;
    // SAFETY: plain window searches with handles Windows gave us.
    while let Ok(next) = unsafe { FindWindowExW(None, after, CLASS, PCWSTR::null()) } {
        if found.len() >= 16 {
            break;
        }
        found.push(next);
        after = Some(next);
    }
    found
}

/// The name of another program that already acts as the tray host (StartAllBack, Seelen UI, ...),
/// if there is one. Explorer itself does not count. We back off instead of fighting it.
pub fn foreign_host() -> Option<String> {
    let ours = OURS.load(Ordering::Relaxed);
    tray_windows()
        .into_iter()
        .filter(|h| h.0 as isize != ours)
        .filter_map(|h| {
            let pid = window_pid(to_32(h));
            process_name(pid)
        })
        .find(|name| !name.eq_ignore_ascii_case("explorer"))
}

/// Explorer's own taskbar window: the `Shell_TrayWnd` that is not ours.
fn explorer() -> Option<HWND> {
    let cached = EXPLORER.load(Ordering::Relaxed);
    // SAFETY: IsWindow accepts any value.
    if cached != 0 && unsafe { IsWindow(Some(hwnd_of(cached))) }.as_bool() {
        return Some(hwnd_of(cached));
    }
    let ours = OURS.load(Ordering::Relaxed);
    let found = tray_windows().into_iter().find(|h| h.0 as isize != ours);
    EXPLORER.store(found.map_or(0, |h| h.0 as isize), Ordering::Relaxed);
    found
}

/// Is ours the window `FindWindow("Shell_TrayWnd")` returns?
fn is_first() -> bool {
    let ours = OURS.load(Ordering::Relaxed);
    // SAFETY: plain call.
    let first = unsafe { FindWindowW(CLASS, PCWSTR::null()) }.unwrap_or_default();
    ours != 0 && first.0 as isize == ours
}

fn raise() {
    // SAFETY: plain call with our own window handle.
    unsafe {
        let _ = SetWindowPos(
            hwnd_of(OURS.load(Ordering::Relaxed)),
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
    RERAISED.fetch_add(1, Ordering::Relaxed);
}

/// Raises our window above Explorer's if `FindWindow` would return Explorer's. Measured
/// (`docs/perf/0.6.md`): the foreground, reorder and Explorer-restart events are enough, a retry
/// timer did not help. The one case we cannot win is while Start is open (see ADR 0026).
fn ensure_first() {
    if OURS.load(Ordering::Relaxed) != 0 && !is_first() {
        raise();
    }
}

/// Some apps read the taskbar's rectangle from `FindWindow("Shell_TrayWnd")` to place their
/// windows; give them Explorer's.
fn mirror_rect() {
    let (Some(target), ours) = (explorer(), OURS.load(Ordering::Relaxed)) else {
        return;
    };
    let mut rect = RECT::default();
    // SAFETY: plain calls; the rect is ours.
    unsafe {
        if GetWindowRect(target, &mut rect).is_ok() {
            let _ = SetWindowPos(
                hwnd_of(ours),
                None,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
        }
    }
}

fn send(event: HostEvent) {
    if let Some(sink) = SINK.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        if sink.try_send(event).is_err() {
            DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Hands a sent message on to Explorer and returns its answer (apps read it as success or not).
///
/// # Safety
/// `lparam` must be valid for `msg` the way Windows gave it to us (a `COPYDATASTRUCT` for
/// `WM_COPYDATA`): Windows marshals it again for the other process.
unsafe fn forward(msg: u32, wparam: WPARAM, lparam: LPARAM, answer_if_alone: isize) -> LRESULT {
    let Some(target) = explorer() else {
        return LRESULT(answer_if_alone);
    };
    if msg != WM_COPYDATA && !InSendMessage().as_bool() {
        // A posted message stays a posted message.
        let _ = PostMessageW(Some(target), msg, wparam, lparam);
        return LRESULT(0);
    }
    let mut result = 0usize;
    let answered = SendMessageTimeoutW(
        target,
        msg,
        wparam,
        lparam,
        SMTO_ABORTIFHUNG,
        FORWARD_TIMEOUT_MS,
        Some(&mut result),
    );
    if answered.0 == 0 {
        FORWARD_FAILED.fetch_add(1, Ordering::Relaxed);
        return LRESULT(0);
    }
    LRESULT(result as isize)
}

/// Reads a tray message before Explorer gets it. The sender is blocked until we return, so its
/// window and icon are still alive here.
fn read_tray(bytes: &[u8]) {
    TRAY_MESSAGES.fetch_add(1, Ordering::Relaxed);
    let Some(message) = parse::parse(bytes) else {
        return;
    };
    let mut icon = None;
    if message.action != Action::Delete {
        if let Some(handle) = message.icon_handle.filter(|h| *h != 0) {
            // SAFETY: CopyIcon accepts any handle and fails cleanly on a bad one.
            icon = unsafe { CopyIcon(HICON(handle as i32 as isize as *mut c_void)) }
                .ok()
                .map(OwnedIcon);
        }
    }
    let pid = window_pid(message.hwnd);
    if message.action == Action::Add && pid != 0 {
        watched()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(message.hwnd);
    }
    send(HostEvent::Tray { message, icon, pid });
}

unsafe fn on_copy_data(wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: for WM_COPYDATA Windows hands us a valid COPYDATASTRUCT whose data is a copy in our
    // address space for the duration of this call.
    let data = &*(lparam.0 as *const COPYDATASTRUCT);
    let tray = data.dwData == COPYDATA_TRAY;
    if tray && !data.lpData.is_null() {
        let len = (data.cbData as usize).min(MAX_PAYLOAD);
        read_tray(std::slice::from_raw_parts(data.lpData as *const u8, len));
    }
    // Tray messages answer TRUE when Explorer isn't there, so apps think their icon was added.
    // Other kinds (the AppBar API, icon rects) have their own meaning of the answer: 0.
    forward(WM_COPYDATA, wparam, lparam, isize::from(tray))
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // A panic must not cross this boundary (it would abort the whole bar), and one bad message
    // must never take the bar down (rule 3): log it and answer "no".
    std::panic::catch_unwind(|| handle(hwnd, msg, wparam, lparam)).unwrap_or_else(|_| {
        eprintln!("[tray-host] a message handler panicked (message {msg:#x})");
        LRESULT(0)
    })
}

unsafe fn handle(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let taskbar_created = TASKBAR_CREATED.load(Ordering::Relaxed);
    match msg {
        WM_COPYDATA => on_copy_data(wparam, lparam),
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE | WM_DPICHANGED => {
            mirror_rect();
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        m if m == taskbar_created && m != 0 => {
            // Explorer restarted: forget its old window and take the top place again.
            EXPLORER.store(0, Ordering::Relaxed);
            mirror_rect();
            ensure_first();
            LRESULT(0)
        }
        // Some apps send commands (show desktop) and registered messages to the taskbar.
        // (0x8000 and up: WM_APP and registered messages.)
        WM_COMMAND | WM_USER..=0xFFFF => forward(msg, wparam, lparam, 0),
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn on_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    let _ = std::panic::catch_unwind(|| on_event_inner(event, hwnd, object));
}

fn on_event_inner(event: u32, hwnd: HWND, object: i32) {
    if event == EVENT_OBJECT_DESTROY {
        if object != OBJID_WINDOW.0 {
            return;
        }
        let handle = to_32(hwnd);
        let known = watched()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&handle);
        if known {
            send(HostEvent::WindowGone(handle));
        }
    } else if event == EVENT_OBJECT_REORDER {
        // Fires for every list and tree in every app: only Explorer's own taskbar matters.
        if hwnd.0 as isize == EXPLORER.load(Ordering::Relaxed) {
            ensure_first();
        }
    } else {
        ensure_first();
    }
}

/// The running host. Dropping it closes the window, so messages go to Explorer alone again.
pub struct Host {
    thread: Option<JoinHandle<()>>,
}

impl Drop for Host {
    fn drop(&mut self) {
        let ours = OURS.load(Ordering::Relaxed);
        if ours != 0 {
            // SAFETY: the window belongs to the host thread, which handles WM_CLOSE itself.
            unsafe {
                let _ = PostMessageW(Some(hwnd_of(ours)), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        stop_sink();
        if let Some(watched) = WATCHED.get() {
            watched.lock().unwrap_or_else(|e| e.into_inner()).clear();
        }
    }
}

/// Starts the host on its own thread. Events go to `sink`.
pub fn start(sink: SyncSender<HostEvent>) -> Result<Host, String> {
    {
        let mut slot = SINK.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_some() {
            return Err("tray host already started".to_string());
        }
        *slot = Some(sink);
    }
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
    let thread = std::thread::Builder::new()
        .name("tray-host".into())
        .spawn(move || run(ready_tx))
        .map_err(|e| e.to_string())?;
    match ready_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(Host {
            thread: Some(thread),
        }),
        Ok(Err(e)) => {
            stop_sink();
            Err(e)
        }
        Err(_) => {
            stop_sink();
            Err("tray host did not start".to_string())
        }
    }
}

/// Forgets the sink so the next `start` is accepted.
fn stop_sink() {
    *SINK.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn run(ready: mpsc::Sender<Result<(), String>>) {
    // SAFETY: standard window setup and message loop on this thread; every handle is ours.
    unsafe {
        let instance: HINSTANCE = match GetModuleHandleW(None) {
            Ok(module) => HINSTANCE(module.0),
            Err(e) => return fail(&ready, e),
        };
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            lpszClassName: CLASS,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return fail(&ready, windows::core::Error::from_thread());
        }
        // Never shown: FindWindow and messages work on hidden windows too.
        let hwnd = match CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            CLASS,
            w!(""),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance),
            None,
        ) {
            Ok(hwnd) => hwnd,
            Err(e) => return fail(&ready, e),
        };
        OURS.store(hwnd.0 as isize, Ordering::Relaxed);
        TASKBAR_CREATED.store(RegisterWindowMessageW(w!("TaskbarCreated")), Ordering::Relaxed);
        // Elevated apps may send to us too (they are blocked by default, UIPI).
        let _ = ChangeWindowMessageFilterEx(hwnd, WM_COPYDATA, MSGFLT_ALLOW, None);
        mirror_rect();
        ensure_first();
        // Cheap events that can change who is on top, and the end of a window that owns icons.
        let hooks: Vec<HWINEVENTHOOK> = [
            EVENT_SYSTEM_FOREGROUND,
            EVENT_OBJECT_REORDER,
            EVENT_OBJECT_DESTROY,
        ]
        .into_iter()
        .map(|event| {
            SetWinEventHook(event, event, None, Some(on_event), 0, 0, WINEVENT_OUTOFCONTEXT)
        })
        .collect();
        let _ = ready.send(Ok(()));

        // Apps already running re-add their icons when they hear this (that is how they survive an
        // Explorer restart); now those messages come through us.
        announce();

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        for hook in hooks.into_iter().filter(|h| !h.is_invalid()) {
            let _ = UnhookWinEvent(hook);
        }
        OURS.store(0, Ordering::Relaxed);
        // Free the class name so a later start (switching the tray back on) can register it again.
        let _ = UnregisterClassW(CLASS, Some(instance));
    }
}

fn fail(ready: &mpsc::Sender<Result<(), String>>, error: windows::core::Error) {
    let _ = ready.send(Err(error.to_string()));
}
