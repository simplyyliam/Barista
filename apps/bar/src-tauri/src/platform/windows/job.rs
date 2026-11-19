//! A Windows Job Object: a group of processes that Windows can kill together.
//!
//! A script can start other programs, and those can start more. Killing only the script would
//! leave them running. So every script starts inside a job set to "kill everything when the job
//! handle closes". Dropping the `Job` (timeout, script removed, bar exit, even a bar crash:
//! Windows closes the handle for us) ends the whole tree.
//!
//! The catch: a program could start a child before we put it in the job, and that child would
//! escape. So `spawn_suspended` + `Job::add` + `resume` is the order: the process is created
//! frozen, goes into the job, and only then starts running (`resume`).

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject, TerminateJobObject,
    JobObjectExtendedLimitInformation, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

/// `CREATE_SUSPENDED | CREATE_NO_WINDOW`: start frozen, and never flash a console window.
pub const CREATION_FLAGS: u32 = 0x0000_0004 | 0x0800_0000;

pub struct Job(HANDLE);

// SAFETY: a job handle is just a number the kernel looks up. Any thread may use or close it.
unsafe impl Send for Job {}
unsafe impl Sync for Job {}

impl Job {
    pub fn new() -> Result<Self, String> {
        // SAFETY: no security attributes and no name; we own the returned handle (closed in Drop).
        let handle = unsafe { CreateJobObjectW(None, None) }.map_err(|e| e.to_string())?;
        let job = Job(handle);
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: `info` is the struct this info class expects and lives for the call.
        unsafe {
            SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        }
        .map_err(|e| e.to_string())?;
        Ok(job)
    }

    /// Puts a process (by raw handle) in the job. Its children join automatically.
    pub fn add(&self, process: *mut std::ffi::c_void) -> Result<(), String> {
        // SAFETY: `process` is a live process handle owned by the caller's `Child`.
        unsafe { AssignProcessToJobObject(self.0, HANDLE(process)) }.map_err(|e| e.to_string())
    }

    /// Kills every process in the job right now. Safe to call twice.
    pub fn terminate(&self) {
        // SAFETY: our own job handle. A failure means the job is already empty.
        unsafe {
            let _ = TerminateJobObject(self.0, 1);
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // Closing the last handle is what triggers KILL_ON_JOB_CLOSE.
        // SAFETY: we own the handle and nothing uses it after this.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

// ntdll is always loaded. `NtResumeProcess` is what Process Explorer's "Resume" calls: it resumes every
// thread of a process in one call. The documented way to find a new process's main thread is a system-wide
// thread snapshot, which cost about 80 ms of CPU per script run (measured), so it is not used.
#[link(name = "ntdll")]
extern "system" {
    fn NtResumeProcess(process: HANDLE) -> i32;
}

/// Unfreezes a process made with `CREATE_SUSPENDED` (by its raw handle, which needs the
/// suspend/resume right: the handle `CreateProcess` returns has all rights).
pub fn resume(process: *mut std::ffi::c_void) -> Result<(), String> {
    // SAFETY: `process` is a live process handle owned by the caller's `Child`.
    let status = unsafe { NtResumeProcess(HANDLE(process)) };
    // NTSTATUS: zero or positive is success.
    if status >= 0 {
        Ok(())
    } else {
        Err(format!("could not start the script process (status {status:#x})"))
    }
}
