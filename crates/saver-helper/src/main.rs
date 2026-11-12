//! Barista helper service (ADR 0020). Windows keeps the energy saver switch in
//! `HKLM\SYSTEM\CurrentControlSet\Control\Power\EnergySaverState` (1 = on, 2 = off) and only
//! administrators may write it. This service runs as LocalSystem, installed once, and does exactly
//! one thing: it sets that value when the bar asks.
//!
//! There is no pipe, no message format and no text to parse. The bar sends a *service control
//! code* (`ControlService`), and the service's access list lets interactive users do that and
//! nothing else. Only two codes mean anything; every other code is refused. So the worst a
//! local program can do through it is flip energy saver, which the signed-in user may do anyway
//! from Quick Settings.
//!
//! The install script puts the exe under Program Files, where only administrators can write: a
//! service that runs a file anybody can replace would be a way to become administrator.

/// The control codes the bar may send (user-defined codes run from 128 to 255).
pub const CODE_SAVER_ON: u32 = 128;
pub const CODE_SAVER_OFF: u32 = 129;

/// The `EnergySaverState` value for a control code; `None` for any other code.
pub fn saver_value(code: u32) -> Option<u32> {
    match code {
        CODE_SAVER_ON => Some(1),
        CODE_SAVER_OFF => Some(2),
        _ => None,
    }
}

#[cfg(windows)]
mod service {
    use super::saver_value;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows::core::{w, PWSTR};
    use windows::Win32::Foundation::{ERROR_CALL_NOT_IMPLEMENTED, ERROR_SUCCESS, HANDLE, NO_ERROR};
    use windows::Win32::System::Registry::{RegSetKeyValueW, HKEY_LOCAL_MACHINE, REG_DWORD};
    use windows::Win32::System::Services::{
        RegisterServiceCtrlHandlerExW, SetServiceStatus, StartServiceCtrlDispatcherW, SERVICE_ACCEPT_STOP,
        SERVICE_CONTROL_INTERROGATE, SERVICE_CONTROL_STOP, SERVICE_RUNNING, SERVICE_START_PENDING,
        SERVICE_STATUS, SERVICE_STATUS_HANDLE, SERVICE_STOPPED, SERVICE_TABLE_ENTRYW,
        SERVICE_WIN32_OWN_PROCESS,
    };
    use windows::Win32::System::Threading::{CreateEventW, SetEvent, WaitForSingleObject, INFINITE};

    static STATUS: AtomicIsize = AtomicIsize::new(0);
    static STOP: AtomicIsize = AtomicIsize::new(0);

    fn report(state: windows::Win32::System::Services::SERVICE_STATUS_CURRENT_STATE, accept: u32) {
        let status = SERVICE_STATUS {
            dwServiceType: SERVICE_WIN32_OWN_PROCESS,
            dwCurrentState: state,
            dwControlsAccepted: accept,
            ..SERVICE_STATUS::default()
        };
        // SAFETY: the handle came from RegisterServiceCtrlHandlerExW; the struct lives for the call.
        unsafe {
            let _ = SetServiceStatus(SERVICE_STATUS_HANDLE(STATUS.load(Ordering::SeqCst) as _), &status);
        }
    }

    fn write_saver(value: u32) {
        // SAFETY: constant strings; the four value bytes live for the call.
        let status = unsafe {
            RegSetKeyValueW(
                HKEY_LOCAL_MACHINE,
                w!(r"SYSTEM\CurrentControlSet\Control\Power"),
                w!("EnergySaverState"),
                REG_DWORD.0,
                Some(std::ptr::from_ref(&value).cast()),
                size_of::<u32>() as u32,
            )
        };
        if status != ERROR_SUCCESS {
            eprintln!("barista-helper: could not write EnergySaverState ({status:?})");
        }
    }

    unsafe extern "system" fn handler(control: u32, _event: u32, _data: *mut core::ffi::c_void, _ctx: *mut core::ffi::c_void) -> u32 {
        match control {
            SERVICE_CONTROL_INTERROGATE => NO_ERROR.0,
            SERVICE_CONTROL_STOP => {
                report(SERVICE_STOPPED, 0);
                // SAFETY: the event was created in `service_main` and stays open until the process ends.
                unsafe {
                    let _ = SetEvent(HANDLE(STOP.load(Ordering::SeqCst) as _));
                }
                NO_ERROR.0
            }
            code => match saver_value(code) {
                Some(value) => {
                    write_saver(value);
                    NO_ERROR.0
                }
                // Anything else, from anybody, is refused.
                None => ERROR_CALL_NOT_IMPLEMENTED.0,
            },
        }
    }

    unsafe extern "system" fn service_main(_argc: u32, _argv: *mut PWSTR) {
        // SAFETY: plain calls; the handler is a function with the right signature.
        unsafe {
            let Ok(status) = RegisterServiceCtrlHandlerExW(w!("BaristaSaver"), Some(handler), None) else {
                return;
            };
            STATUS.store(status.0 as isize, Ordering::SeqCst);
            report(SERVICE_START_PENDING, 0);
            let Ok(stop) = CreateEventW(None, true, false, None) else {
                report(SERVICE_STOPPED, 0);
                return;
            };
            STOP.store(stop.0 as isize, Ordering::SeqCst);
            report(SERVICE_RUNNING, SERVICE_ACCEPT_STOP);
            WaitForSingleObject(stop, INFINITE);
        }
    }

    pub fn run() {
        let table = [
            SERVICE_TABLE_ENTRYW { lpServiceName: PWSTR(w!("BaristaSaver").as_ptr() as *mut u16), lpServiceProc: Some(service_main) },
            SERVICE_TABLE_ENTRYW::default(),
        ];
        // SAFETY: the table ends with a null entry, as the API needs.
        if unsafe { StartServiceCtrlDispatcherW(table.as_ptr()) }.is_err() {
            eprintln!("barista-helper: this program is a Windows service; install it with `pnpm helper:install`");
        }
    }
}

fn main() {
    #[cfg(windows)]
    service::run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_two_codes_mean_anything() {
        assert_eq!(saver_value(CODE_SAVER_ON), Some(1));
        assert_eq!(saver_value(CODE_SAVER_OFF), Some(2));
        for code in (0..=255u32).filter(|c| *c != 128 && *c != 129) {
            assert_eq!(saver_value(code), None, "code {code}");
        }
        assert_eq!(saver_value(u32::MAX), None);
    }
}
