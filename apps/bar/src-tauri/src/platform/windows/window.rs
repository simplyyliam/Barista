//! The bar window's Win32 side: styles, placement, show/hide, and a hook that turns the
//! window messages we care about into plain `WindowEvent`s for the rest of the app.

use super::appbar;
use crate::bar::{covers, Rect};
use crate::radius::{snapped_by_geometry, WindowState};
use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowLongPtrW, PBT_APMPOWERSTATUSCHANGE, PBT_POWERSETTINGCHANGE, GetWindowRect, IsIconic, IsZoomed, RegisterWindowMessageW, SendMessageW,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST,
    SPI_SETDESKWALLPAPER, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE, SW_SHOWNOACTIVATE, WM_ACTIVATE,
    WM_DISPLAYCHANGE, WM_DPICHANGED, WM_HOTKEY, WM_NCACTIVATE, WM_POWERBROADCAST, WM_SETTINGCHANGE, WM_TIMECHANGE, WM_WINDOWPOSCHANGED, WS_CAPTION,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

/// What happened to the screen or the shell, in our own words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowEvent {
    /// Another appbar (or the taskbar) moved; our strip may need to move too.
    PosChanged,
    /// Windows says a full-screen app started (`true`) or ended (`false`).
    FullscreenApp(bool),
    DisplayChanged,
    DpiChanged,
    /// Explorer restarted and forgot our appbar.
    TaskbarCreated,
    /// The desktop wallpaper changed (Settings, a theme, a slideshow).
    WallpaperChanged,
    /// The debug hotkey for forcing the bar tone was pressed (only registered in debug builds).
    ToneHotkey,
    /// The clock or time zone was changed (`WM_TIMECHANGE`).
    TimeChanged,
    /// Regional settings changed (`WM_SETTINGCHANGE` with "intl"): 12/24 h, first weekday.
    RegionChanged,
    /// Battery level, power source or battery saver changed.
    PowerChanged,
}

// ABN_* notification codes, sent in WPARAM with `appbar::CALLBACK_MSG`.
const ABN_POSCHANGED: usize = 1;
const ABN_FULLSCREENAPP: usize = 2;

static HANDLER: OnceLock<fn(WindowEvent)> = OnceLock::new();
/// Id of the "TaskbarCreated" message Explorer broadcasts after it restarts.
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

fn hwnd_from(raw: isize) -> HWND {
    HWND(raw as *mut _)
}

/// Makes the window never take focus and keeps it out of Alt+Tab and the taskbar.
/// Call while the window is still hidden.
pub fn set_no_focus_styles(hwnd: HWND) {
    // SAFETY: plain get/set of the window's extended style bits.
    unsafe {
        let old = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let add = (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, old | add);
    }
}

/// Moves and sizes the window (physical pixels) above normal windows, without activating it.
pub fn place(hwnd: HWND, r: Rect) {
    // SAFETY: plain call; failure (window gone during exit) is harmless, so it is ignored.
    let _ = unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            r.left,
            r.top,
            r.width(),
            r.height(),
            SWP_NOACTIVATE,
        )
    };
}

/// Styles for a menu window: no taskbar button, round corners, no Windows show/hide animation
/// (menus open instantly) and the system acrylic backdrop (ADR 0013). `no_activate` keeps a
/// submenu window from ever taking focus. Returns whether acrylic is available (Win11 22H2+);
/// without it the UI draws an opaque panel.
pub fn set_menu_styles(hwnd: HWND, no_activate: bool) -> bool {
    use windows::Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMSBT_TRANSIENTWINDOW,
        DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_TRANSITIONS_FORCEDISABLED,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };
    use windows::Win32::UI::Controls::MARGINS;
    // SAFETY: every attribute value is a plain value that lives for the whole call. Older
    // Windows does not know an attribute and returns an error, which we treat as "not there".
    unsafe {
        let round = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::addr_of!(round).cast(),
            std::mem::size_of_val(&round) as u32,
        );
        let no_anim: i32 = 1;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            std::ptr::addr_of!(no_anim).cast(),
            std::mem::size_of_val(&no_anim) as u32,
        );
        let backdrop = DWMSBT_TRANSIENTWINDOW;
        let acrylic = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            std::ptr::addr_of!(backdrop).cast(),
            std::mem::size_of_val(&backdrop) as u32,
        )
        .is_ok();
        if acrylic {
            // The backdrop only shows if the frame is extended over the whole window. Without
            // acrylic (Windows 10) we leave the window alone: the UI draws an opaque panel.
            let margins = MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            };
            let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
        }
        let mut extra = WS_EX_TOOLWINDOW.0 as isize;
        if no_activate {
            extra |= WS_EX_NOACTIVATE.0 as isize;
        }
        let old = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, old | extra);
        acrylic
    }
}

/// Switches the panel window between the menu look (`glass`: system backdrop, round corners) and a
/// bare transparent window (the page draws its own cards). Can be called again at any time.
/// The shadow is Tauri's `set_shadow`, done by the caller.
pub fn set_panel_chrome(hwnd: HWND, glass: bool) {
    use windows::Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMSBT_NONE, DWMSBT_TRANSIENTWINDOW,
        DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
        DWMWCP_ROUND,
    };
    use windows::Win32::UI::Controls::MARGINS;
    let corner = if glass { DWMWCP_ROUND } else { DWMWCP_DONOTROUND };
    let backdrop = if glass { DWMSBT_TRANSIENTWINDOW } else { DWMSBT_NONE };
    // -1 extends the frame over the whole window so the backdrop shows; 0 takes it back.
    let m = if glass { -1 } else { 0 };
    let margins = MARGINS {
        cxLeftWidth: m,
        cxRightWidth: m,
        cyTopHeight: m,
        cyBottomHeight: m,
    };
    // SAFETY: plain values that live for the call. Older Windows does not know an attribute and
    // returns an error, which is fine: that window just keeps what it has.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::addr_of!(corner).cast(),
            std::mem::size_of_val(&corner) as u32,
        );
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            std::ptr::addr_of!(backdrop).cast(),
            std::mem::size_of_val(&backdrop) as u32,
        );
        let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);
    }
}

/// Shows `hwnd` without activating it and without anyone seeing it (cloaked), so Windows and
/// WebView2 build the window's surfaces now instead of during the user's first click.
pub fn warm_show(hwnd: HWND) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
    let on: i32 = 1;
    // SAFETY: a plain i32 that lives for the call, and a window handle that belongs to us.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK,
            std::ptr::addr_of!(on).cast(),
            std::mem::size_of_val(&on) as u32,
        );
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
}

/// Ends `warm_show`: hides the window again (unless `keep` says it was really opened meanwhile) and
/// takes the cloak off.
pub fn warm_end(hwnd: HWND, keep: bool) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK};
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    let off: i32 = 0;
    // SAFETY: as in `warm_show`.
    unsafe {
        if !keep {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK,
            std::ptr::addr_of!(off).cast(),
            std::mem::size_of_val(&off) as u32,
        );
    }
}

/// Makes DWM draw the window as "active" without activating it. The system acrylic turns flat
/// on an inactive window, and the submenu window can never be active (it would take the keyboard
/// from the main menu), so it gets the active look this way.
pub fn appear_active(hwnd: HWND) {
    // SAFETY: WM_NCACTIVATE with plain values; it only changes how the frame is drawn.
    unsafe {
        let _ = SendMessageW(hwnd, WM_NCACTIVATE, Some(WPARAM(1)), Some(LPARAM(0)));
    }
}

/// How a menu window's frame differs from what is drawn. Windows keeps an invisible border around
/// a window with a shadow, so the window rect is bigger than what you see (the "visible frame").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameMetrics {
    /// Window rect to visible frame: (left, top, right, bottom) px of invisible border.
    pub inset: (i32, i32, i32, i32),
    /// Visible frame minus client area: (width, height) px, a thin rim around the page.
    pub extra: (i32, i32),
}

/// Measures the frame of `hwnd`. Works while the window is hidden. `None` if Windows can't say.
pub fn frame_metrics(hwnd: HWND) -> Option<FrameMetrics> {
    let (mut window, mut client, mut visible) = (RECT::default(), RECT::default(), RECT::default());
    // SAFETY: each call only writes into the local RECT it is given (the DWM call gets its size).
    unsafe {
        GetWindowRect(hwnd, &mut window).ok()?;
        GetClientRect(hwnd, &mut client).ok()?;
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            std::ptr::addr_of_mut!(visible).cast(),
            std::mem::size_of::<RECT>() as u32,
        )
        .ok()?;
    }
    let (vw, vh) = (visible.right - visible.left, visible.bottom - visible.top);
    let inset = (
        visible.left - window.left,
        visible.top - window.top,
        window.right - visible.right,
        window.bottom - visible.bottom,
    );
    // A window that was never laid out reports nothing useful: don't trust it.
    let sane = vw > 0 && vh > 0 && [inset.0, inset.1, inset.2, inset.3].iter().all(|v| (0..=64).contains(v));
    sane.then_some(FrameMetrics {
        inset,
        extra: (vw - (client.right - client.left), vh - (client.bottom - client.top)),
    })
}

/// Puts the window's outer rect exactly here, without activating it.
pub fn set_outer_rect(hwnd: HWND, left: i32, top: i32, width: i32, height: i32) {
    // SAFETY: plain call with values; no pointers.
    unsafe {
        let _ = SetWindowPos(hwnd, None, left, top, width, height, SWP_NOACTIVATE | SWP_NOZORDER);
    }
}

/// Windows' own show/hide animation for a menu window. Off while it opens (instant), on for the
/// moment it hides, so the whole window, glass and shadow included, fades out like a native menu.
pub fn set_menu_transitions(hwnd: HWND, enabled: bool) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED};
    let forced_off: i32 = (!enabled).into();
    // SAFETY: plain i32 that lives for the whole call.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            std::ptr::addr_of!(forced_off).cast(),
            std::mem::size_of_val(&forced_off) as u32,
        );
    }
}

/// Picks the dark or the light acrylic tint for a menu window.
pub fn set_menu_dark(hwnd: HWND, dark: bool) {
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
    let value: i32 = dark.into();
    // SAFETY: plain i32 that lives for the whole call.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            std::ptr::addr_of!(value).cast(),
            std::mem::size_of_val(&value) as u32,
        );
    }
}

/// The mouse position's x in physical screen pixels, if Windows can tell.
pub fn cursor_x() -> Option<i32> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT::default();
    // SAFETY: GetCursorPos only writes into our local POINT.
    unsafe { GetCursorPos(&mut p) }.ok().map(|()| p.x)
}

pub fn set_visible(hwnd: HWND, visible: bool) {
    // SAFETY: plain call. SW_SHOWNOACTIVATE shows without stealing focus.
    unsafe {
        let _ = ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
    }
}

/// Starts listening to the window's messages. `handler` runs on the UI thread.
pub fn install(raw_hwnd: isize, handler: fn(WindowEvent)) {
    let _ = HANDLER.set(handler);
    // SAFETY: registering a message name is always allowed; the id is process-wide.
    let id = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    TASKBAR_CREATED.store(id, Ordering::SeqCst);
    // SAFETY: the subclass proc below has the exact signature Windows expects. Subclassing
    // puts us in front of Tauri's own handler: we see each message first and pass it on.
    unsafe {
        let _ = SetWindowSubclass(hwnd_from(raw_hwnd), Some(subclass_proc), 0xBA_0001, 0);
    }
}

/// Debug builds: Ctrl+Alt+Shift+T cycles the bar tone (auto, light, dark). Windows delivers the
/// key to the bar even though the bar never takes focus. A taken key is only logged.
#[cfg(debug_assertions)]
pub fn register_tone_hotkey(raw_hwnd: isize) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT,
    };
    let modifiers = MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_NOREPEAT;
    // SAFETY: the window is ours and lives for the whole run; id 1 is only used here.
    let result =
        unsafe { RegisterHotKey(Some(hwnd_from(raw_hwnd)), 1, modifiers, u32::from(b'T')) };
    if let Err(e) = result {
        eprintln!("barista: tone hotkey not registered: {e}");
    }
}

/// `WM_SETTINGCHANGE` carries the name of the changed setting as text in lparam.
fn lparam_is_intl(lparam: LPARAM) -> bool {
    if lparam.0 == 0 {
        return false;
    }
    // SAFETY: for this message Windows passes a null or a NUL-terminated wide string that is
    // valid while we handle it.
    let name = unsafe { windows::core::PCWSTR(lparam.0 as *const u16).to_string() };
    matches!(name.as_deref(), Ok("intl") | Ok("TimeZoneInformation"))
}

fn emit(event: WindowEvent) {
    if let Some(handler) = HANDLER.get() {
        // A panic must not unwind through Windows' code (that aborts the process), so catch
        // it. The panic hook has already given the screen space back by then.
        let _ = std::panic::catch_unwind(|| handler(event));
    }
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    // Tauri handles the message first. This matters for WM_DPICHANGED: Tauri resizes the
    // window to Windows' suggestion, and only then do we put it back to our own rectangle.
    // SAFETY: forwarding the same arguments we were given.
    let result = unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) };

    match msg {
        m if m == appbar::CALLBACK_MSG => match wparam.0 {
            ABN_POSCHANGED => emit(WindowEvent::PosChanged),
            ABN_FULLSCREENAPP => emit(WindowEvent::FullscreenApp(lparam.0 != 0)),
            _ => {}
        },
        WM_DISPLAYCHANGE => emit(WindowEvent::DisplayChanged),
        WM_DPICHANGED => emit(WindowEvent::DpiChanged),
        WM_SETTINGCHANGE if wparam.0 == SPI_SETDESKWALLPAPER.0 as usize => {
            emit(WindowEvent::WallpaperChanged)
        }
        WM_SETTINGCHANGE if lparam_is_intl(lparam) => emit(WindowEvent::RegionChanged),
        WM_TIMECHANGE => emit(WindowEvent::TimeChanged),
        WM_POWERBROADCAST
            if wparam.0 == PBT_POWERSETTINGCHANGE as usize
                || wparam.0 == PBT_APMPOWERSTATUSCHANGE as usize =>
        {
            emit(WindowEvent::PowerChanged)
        }
        WM_HOTKEY => emit(WindowEvent::ToneHotkey),
        WM_ACTIVATE => appbar::activate(hwnd),
        WM_WINDOWPOSCHANGED => appbar::window_pos_changed(hwnd),
        m if m != 0 && m == TASKBAR_CREATED.load(Ordering::SeqCst) => {
            emit(WindowEvent::TaskbarCreated)
        }
        _ => {}
    }
    result
}

fn rect_from(r: RECT) -> Rect {
    Rect {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}

/// The monitor the window is mostly on: its full rectangle and its work area (physical pixels).
fn monitor_rects(hwnd: HWND) -> Option<(Rect, Rect)> {
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: `info` has its size filled in and lives through the call.
    let ok = unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        GetMonitorInfoW(monitor, &mut info).as_bool()
    };
    ok.then(|| (rect_from(info.rcMonitor), rect_from(info.rcWork)))
}

/// The window's outer rectangle in physical pixels.
fn window_rect(hwnd: HWND) -> Option<Rect> {
    let mut r = RECT::default();
    // SAFETY: `r` is a valid out-pointer.
    unsafe { GetWindowRect(hwnd, &mut r) }.ok()?;
    Some(rect_from(r))
}

/// The visible frame, without the invisible resize border and shadow that `GetWindowRect`
/// includes. Snapped windows are flush with the work area only when measured this way.
fn frame_bounds(hwnd: HWND) -> Option<Rect> {
    let mut r = RECT::default();
    // SAFETY: Windows writes one RECT into the buffer we pass, and we pass its exact size.
    let result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            (&mut r as *mut RECT).cast::<c_void>(),
            size_of::<RECT>() as u32,
        )
    };
    match result {
        Ok(()) => Some(rect_from(r)),
        Err(_) => window_rect(hwnd),
    }
}

/// Is the window docked to a half or quarter of the screen? Windows has no call for this, so we
/// compare the window's frame with the monitor's work area (see `radius::snapped_by_geometry`).
pub fn is_snapped(hwnd: HWND) -> bool {
    // SAFETY: plain query.
    if unsafe { IsZoomed(hwnd).as_bool() || IsIconic(hwnd).as_bool() } {
        return false; // maximized and minimized windows are not "snapped"
    }
    match (frame_bounds(hwnd), monitor_rects(hwnd)) {
        (Some(frame), Some((_, work))) => snapped_by_geometry(&frame, &work),
        _ => false,
    }
}

/// What the window is doing, for the corner radius. Order matters:
///
/// 1. **Fullscreen**: it covers the whole monitor (not just the work area) and has no title bar
///    (a maximized window keeps its caption, even when an auto-hidden taskbar lets it cover the
///    whole monitor).
/// 2. **Maximized**: Windows says so.
/// 3. **Snapped**: docked to a half or quarter.
/// 4. Otherwise a normal window. `None` when the window can't be queried (it is gone).
pub fn window_state(hwnd: HWND) -> Option<WindowState> {
    let rect = window_rect(hwnd)?;
    let (monitor, _) = monitor_rects(hwnd)?;
    // SAFETY: plain query.
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } as u32;
    let has_caption = style & WS_CAPTION.0 == WS_CAPTION.0;
    Some(if covers(&rect, &monitor) && !has_caption {
        WindowState::Fullscreen
    } else if unsafe { IsZoomed(hwnd).as_bool() } {
        WindowState::Maximized
    } else if is_snapped(hwnd) {
        WindowState::Snapped
    } else {
        WindowState::Normal
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::w;
    use windows::Win32::UI::HiDpi::{
        SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, SetWindowLongPtrW, SWP_NOZORDER, SW_MAXIMIZE, SW_RESTORE,
        WS_OVERLAPPEDWINDOW, WS_POPUP, WS_VISIBLE,
    };

    /// A real top-level window (the system's own "STATIC" class, so nothing to register), shown
    /// without taking focus. Destroyed when dropped.
    struct TestWindow(HWND);

    impl TestWindow {
        fn new() -> Option<TestWindow> {
            // The app's manifest makes it per-monitor DPI aware, so every rectangle is in physical
            // pixels. A test binary has no such manifest: do the same for this thread, or
            // Windows would hand out scaled coordinates that do not match DWM's.
            // SAFETY: plain call, only changes this thread's DPI mode.
            unsafe {
                SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            }
            // SAFETY: plain window creation with a predefined class and no parent.
            let hwnd = unsafe {
                CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    w!("STATIC"),
                    w!("barista test window"),
                    WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                    200,
                    200,
                    600,
                    400,
                    None,
                    None,
                    None,
                    None,
                )
            }
            .ok()?;
            // SAFETY: valid window handle.
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            Some(TestWindow(hwnd))
        }

        fn place(&self, r: Rect) {
            // SAFETY: valid window handle.
            unsafe {
                let _ = SetWindowPos(
                    self.0,
                    None,
                    r.left,
                    r.top,
                    r.width(),
                    r.height(),
                    SWP_NOACTIVATE | SWP_NOZORDER,
                );
            }
        }

        /// Places the *visible frame* (without the invisible resize border) at `r`, which is how
        /// Windows places a snapped window.
        fn place_frame(&self, r: Rect) {
            self.place(r);
            let (outer, frame) = (window_rect(self.0).unwrap(), frame_bounds(self.0).unwrap());
            self.place(Rect {
                left: r.left - (frame.left - outer.left),
                top: r.top - (frame.top - outer.top),
                right: r.right + (outer.right - frame.right),
                bottom: r.bottom + (outer.bottom - frame.bottom),
            });
        }
    }

    impl Drop for TestWindow {
        fn drop(&mut self) {
            // SAFETY: we created this window and destroy it once.
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }

    #[test]
    fn window_state_follows_the_real_window() {
        let Some(win) = TestWindow::new() else {
            eprintln!("no desktop to create a window on: skipping");
            return;
        };
        let (monitor, work) = monitor_rects(win.0).expect("monitor");
        let (w, h) = (work.width(), work.height());

        win.place(Rect {
            left: work.left + 100,
            top: work.top + 100,
            right: work.left + 700,
            bottom: work.top + 500,
        });
        assert_eq!(window_state(win.0), Some(WindowState::Normal));
        assert!(!is_snapped(win.0));

        // Left half, right half and a quarter of the work area.
        win.place_frame(Rect {
            left: work.left,
            top: work.top,
            right: work.left + w / 2,
            bottom: work.bottom,
        });
        assert_eq!(window_state(win.0), Some(WindowState::Snapped), "left half");
        win.place_frame(Rect {
            left: work.left + w / 2,
            top: work.top,
            right: work.right,
            bottom: work.bottom,
        });
        assert_eq!(
            window_state(win.0),
            Some(WindowState::Snapped),
            "right half"
        );
        win.place_frame(Rect {
            left: work.left,
            top: work.top,
            right: work.left + w / 2,
            bottom: work.top + h / 2,
        });
        assert_eq!(window_state(win.0), Some(WindowState::Snapped), "quarter");

        // Maximized.
        // SAFETY: valid window handle.
        unsafe {
            let _ = ShowWindow(win.0, SW_MAXIMIZE);
        }
        assert_eq!(window_state(win.0), Some(WindowState::Maximized));
        assert!(!is_snapped(win.0), "a maximized window is not snapped");
        // SAFETY: valid window handle.
        unsafe {
            let _ = ShowWindow(win.0, SW_RESTORE);
        }
        win.place(Rect {
            left: work.left + 100,
            top: work.top + 100,
            right: work.left + 700,
            bottom: work.top + 500,
        });
        assert_eq!(window_state(win.0), Some(WindowState::Normal), "restored");

        // Full screen: no title bar, covers the whole monitor.
        // SAFETY: valid window handle; replacing the style bits of our own window.
        unsafe {
            SetWindowLongPtrW(win.0, GWL_STYLE, (WS_POPUP | WS_VISIBLE).0 as isize);
        }
        win.place(monitor);
        assert_eq!(window_state(win.0), Some(WindowState::Fullscreen));
    }

    #[test]
    fn a_window_that_is_gone_has_no_state() {
        let win = TestWindow::new();
        let Some(win) = win else { return };
        let hwnd = win.0;
        drop(win);
        assert_eq!(window_state(hwnd), None);
    }
}
