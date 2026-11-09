//! The Win32 side of the named pipe: who may connect, and who is on the other end.

use std::ffi::c_void;
use std::io;
use std::os::windows::io::AsRawHandle;
use std::sync::OnceLock;
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows::core::{HSTRING, PWSTR};
use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{
    GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
    TOKEN_USER,
};
use windows::Win32::System::Pipes::GetNamedPipeClientProcessId;
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

pub const PIPE_NAME: &str = r"\\.\pipe\barista";

fn to_io(error: windows::core::Error) -> io::Error {
    io::Error::other(error)
}

/// The SID of the user running the bar, as text (like `S-1-5-21-...`).
fn current_user_sid() -> io::Result<String> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).map_err(to_io)?;
        // Ask twice: the first call only reports how many bytes the answer needs.
        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        // u64 elements keep the buffer 8-byte aligned, which TOKEN_USER requires.
        let mut buffer = vec![0u64; (needed as usize).div_ceil(8).max(1)];
        let info = GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            (buffer.len() * 8) as u32,
            &mut needed,
        );
        let _ = CloseHandle(token);
        info.map_err(to_io)?;
        let user = &*(buffer.as_ptr().cast::<TOKEN_USER>());
        let mut text = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut text).map_err(to_io)?;
        let sid = text.to_string().map_err(io::Error::other);
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
        sid
    }
}

/// SDDL text for "only this user, full access, nobody else" (the `P` blocks inherited rules).
/// Computed once: the user can't change while the bar runs.
fn user_only_sddl() -> io::Result<&'static str> {
    static SDDL: OnceLock<String> = OnceLock::new();
    if let Some(sddl) = SDDL.get() {
        return Ok(sddl);
    }
    let sddl = format!("D:P(A;;GA;;;{})", current_user_sid()?);
    Ok(SDDL.get_or_init(|| sddl))
}

/// Creates one pipe instance. `first` must be true for the very first one: Windows then
/// refuses if any other process already owns the name (so nobody can squat `barista` before
/// us and read what clients send). Must run inside the tokio runtime.
pub fn create_server(first: bool) -> io::Result<NamedPipeServer> {
    let sddl = user_only_sddl()?;
    unsafe {
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            &HSTRING::from(sddl),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
        .map_err(to_io)?;
        let mut attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: false.into(),
        };
        let server = ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(
                PIPE_NAME,
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast::<c_void>(),
            );
        // The pipe keeps its own copy of the descriptor, so ours can be freed right away.
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
        server
    }
}

/// The process id on the other end of a connected pipe, straight from Windows (a client
/// can't fake it).
pub fn client_pid(server: &NamedPipeServer) -> Option<u32> {
    let mut pid = 0u32;
    let handle = HANDLE(server.as_raw_handle());
    unsafe { GetNamedPipeClientProcessId(handle, &mut pid) }.ok()?;
    Some(pid)
}
