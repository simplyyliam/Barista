//! What to call the program that owns a window: its description from the file's version info
//! ("Notepad"), else the exe name without `.exe`. Used for the app name next to the menus.

use windows::core::{HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, MAX_PATH};
use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GetClassNameW, GetWindowThreadProcessId, GA_ROOT};

/// The top-level window a (possibly child) window belongs to.
pub fn root_window(hwnd: isize) -> isize {
    // SAFETY: plain call.
    unsafe { GetAncestor(HWND(hwnd as *mut _), GA_ROOT).0 as isize }
}

pub fn pid_of(hwnd: isize) -> u32 {
    let mut pid = 0u32;
    // SAFETY: the call only writes the pid we point at.
    unsafe { GetWindowThreadProcessId(HWND(hwnd as *mut _), Some(&mut pid)) };
    pid
}

pub fn class_name(hwnd: isize) -> String {
    let mut buffer = [0u16; 128];
    // SAFETY: the buffer is ours and its length is passed.
    let len = unsafe { GetClassNameW(HWND(hwnd as *mut _), &mut buffer) };
    String::from_utf16_lossy(&buffer[..len.max(0) as usize])
}

/// The exe path of a process, if we may ask.
pub fn exe_path(pid: u32) -> Option<String> {
    // SAFETY: the handle is closed on every path; the buffer is ours.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buffer = vec![0u16; MAX_PATH as usize * 2];
        let mut len = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        result.ok()?;
        Some(String::from_utf16_lossy(&buffer[..len as usize]))
    }
}

/// `FileDescription` from the exe's version info (the first language it lists).
fn file_description(path: &str) -> Option<String> {
    let wide = HSTRING::from(path);
    // SAFETY: the buffers are ours and sized by the calls that fill them.
    unsafe {
        let size = GetFileVersionInfoSizeW(&wide, None);
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(&wide, None, size, data.as_mut_ptr() as *mut _).ok()?;
        let mut translation: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        if !VerQueryValueW(data.as_ptr() as *const _, &HSTRING::from(r"\VarFileInfo\Translation"), &mut translation, &mut len).as_bool()
            || len < 4
        {
            return None;
        }
        let pair = std::slice::from_raw_parts(translation as *const u16, 2);
        let key = HSTRING::from(format!(r"\StringFileInfo\{:04x}{:04x}\FileDescription", pair[0], pair[1]));
        let mut text: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut text_len = 0u32;
        if !VerQueryValueW(data.as_ptr() as *const _, &key, &mut text, &mut text_len).as_bool() || text_len == 0 {
            return None;
        }
        let chars = std::slice::from_raw_parts(text as *const u16, text_len as usize);
        let name = String::from_utf16_lossy(chars).trim_end_matches('\0').trim().to_string();
        let _ = PCWSTR::null();
        (!name.is_empty()).then_some(name)
    }
}

/// A window a user would call "an app window": top-level (no owner), visible, not a tool window and not one
/// that never takes the focus. The shell's popups (Start, search, flyouts) are none of these.
pub fn is_normal_window(hwnd: isize) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindow, GetWindowLongPtrW, GET_WINDOW_CMD, GWL_EXSTYLE, GWL_STYLE, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_VISIBLE,
    };
    let window = HWND(hwnd as *mut _);
    // SAFETY: plain calls on a window handle; a dead window gives zeros and an error.
    unsafe {
        let style = GetWindowLongPtrW(window, GWL_STYLE) as u32;
        let ex_style = GetWindowLongPtrW(window, GWL_EXSTYLE) as u32;
        // GW_OWNER is 4.
        let owner = GetWindow(window, GET_WINDOW_CMD(4)).map(|h| !h.0.is_null()).unwrap_or(false);
        style & WS_VISIBLE.0 != 0 && ex_style & (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) == 0 && !owner
    }
}

/// Programs whose windows are the shell itself (Start, search, the text input flyout, the lock screen), and
/// Explorer's non-folder windows (the desktop, the taskbar). File Explorer's own windows (`CabinetWClass`) are fine.
pub fn is_shell_window(exe_stem: &str, class: &str) -> bool {
    const SHELL_HOSTS: [&str; 6] =
        ["shellexperiencehost", "startmenuexperiencehost", "searchhost", "textinputhost", "lockapp", "searchapp"];
    let stem = exe_stem.to_ascii_lowercase();
    SHELL_HOSTS.contains(&stem.as_str()) || (stem == "explorer" && class != "CabinetWClass")
}

/// "Notepad", "Windows Command Processor", or the exe stem.
pub fn display_name(pid: u32) -> Option<String> {
    let path = exe_path(pid)?;
    file_description(&path).or_else(|| {
        std::path::Path::new(&path).file_stem().map(|s| s.to_string_lossy().into_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_windows_are_told_from_app_windows() {
        assert!(is_shell_window("StartMenuExperienceHost", "Windows.UI.Core.CoreWindow"));
        assert!(is_shell_window("SearchHost", "Windows.UI.Core.CoreWindow"));
        assert!(is_shell_window("explorer", "Shell_TrayWnd"));
        assert!(is_shell_window("explorer", "XamlExplorerHostIslandWindow"));
        // A File Explorer folder window is an app window.
        assert!(!is_shell_window("explorer", "CabinetWClass"));
        assert!(!is_shell_window("notepad", "Notepad"));
        assert!(!is_shell_window("Zed", "Zed::Window"));
    }

    #[test]
    fn a_missing_window_is_not_a_normal_window() {
        assert!(!is_normal_window(0));
    }
}
