//! Small shell helpers: open a URI, and the session actions (lock, sleep, restart, ...).

use windows::core::HSTRING;
use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LookupPrivilegeValueW, LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED,
    TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Power::SetSuspendState;
use windows::Win32::System::Shutdown::{
    ExitWindowsEx, LockWorkStation, EWX_LOGOFF, EWX_POWEROFF, EWX_REBOOT, EXIT_WINDOWS_FLAGS,
    SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Opens a `ms-settings:` page or another URI the shell knows.
pub fn open_uri(uri: &str) {
    // SAFETY: strings live for the call; no window handle, no working directory.
    unsafe {
        ShellExecuteW(
            None,
            &HSTRING::from("open"),
            &HSTRING::from(uri),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}

pub fn lock() {
    // SAFETY: no arguments; locks the current session.
    let _ = unsafe { LockWorkStation() };
}

pub fn sleep() {
    // Sleep (not hibernate), don't force apps, don't disable wake events.
    // SAFETY: plain call.
    let _ = unsafe { SetSuspendState(false, false, false) };
}

/// Windows asks apps to close and lets them cancel (no EWX_FORCE), so unsaved work can stop it.
fn exit_windows(flags: EXIT_WINDOWS_FLAGS) {
    // SAFETY: the token handle is closed again; the privilege struct lives for the call.
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
        .is_ok()
        {
            let mut luid = LUID::default();
            if LookupPrivilegeValueW(None, &HSTRING::from("SeShutdownPrivilege"), &mut luid).is_ok() {
                let privileges = TOKEN_PRIVILEGES {
                    PrivilegeCount: 1,
                    Privileges: [LUID_AND_ATTRIBUTES {
                        Luid: luid,
                        Attributes: SE_PRIVILEGE_ENABLED,
                    }],
                };
                let _ = AdjustTokenPrivileges(token, false, Some(&privileges), 0, None, None);
            }
            let _ = CloseHandle(token);
        }
        let _ = ExitWindowsEx(flags, SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_FLAG_PLANNED);
    }
}

pub fn restart() {
    exit_windows(EWX_REBOOT);
}

pub fn shut_down() {
    exit_windows(EWX_POWEROFF);
}

pub fn sign_out() {
    exit_windows(EWX_LOGOFF);
}
