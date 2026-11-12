//! Talks to the Barista helper service (`crates/saver-helper`, ADR 0020), which can write the one
//! setting that needs administrator rights: the energy saver switch. We never send data, only a
//! service control code, and the helper only knows two of them.

use windows::core::w;
use windows::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QueryServiceStatus, SC_HANDLE,
    SC_MANAGER_CONNECT, SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_STATUS,
    SERVICE_USER_DEFINED_CONTROL,
};

/// The control codes the helper understands (kept in step with `crates/saver-helper`).
const CODE_SAVER_ON: u32 = 128;
const CODE_SAVER_OFF: u32 = 129;

/// Closes a service handle when it goes out of scope.
struct Handle(SC_HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle was opened by us and is closed once.
        unsafe {
            let _ = CloseServiceHandle(self.0);
        }
    }
}

fn open(access: u32) -> Option<(Handle, Handle)> {
    // SAFETY: constant name; both handles are closed by `Handle`.
    unsafe {
        let scm = Handle(OpenSCManagerW(None, None, SC_MANAGER_CONNECT).ok()?);
        let service = Handle(OpenServiceW(scm.0, w!("BaristaSaver"), access).ok()?);
        Some((scm, service))
    }
}

/// True when the helper is installed and running (and we may talk to it).
pub fn available() -> bool {
    let Some((_scm, service)) = open(SERVICE_QUERY_STATUS) else {
        return false;
    };
    let mut status = SERVICE_STATUS::default();
    // SAFETY: the struct lives for the call.
    unsafe { QueryServiceStatus(service.0, &mut status) }.is_ok() && status.dwCurrentState == SERVICE_RUNNING
}

/// Asks the helper to switch energy saver. False if it isn't there or refused.
pub fn set_saver(on: bool) -> bool {
    let Some((_scm, service)) = open(SERVICE_USER_DEFINED_CONTROL) else {
        return false;
    };
    let mut status = SERVICE_STATUS::default();
    let code = if on { CODE_SAVER_ON } else { CODE_SAVER_OFF };
    // SAFETY: the struct lives for the call.
    unsafe { ControlService(service.0, code, &mut status) }.is_ok()
}
