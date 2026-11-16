//! Start with Windows: one value in the user's Run key (no admin, no scheduled task).
//! The registry is the truth: Task Manager's Startup tab can switch it off behind our back,
//! so the settings window reads it instead of trusting a saved flag (ADR 0031).

use std::path::Path;
use windows::core::{w, HSTRING};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
};

const KEY: windows::core::PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const NAME: windows::core::PCWSTR = w!("Barista");

/// The value Windows runs: the exe path in quotes, so a path with spaces still works.
pub fn run_value(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// True when our Run value exists and points at this exe (an old copy elsewhere does not count).
pub fn is_enabled() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    read().is_some_and(|v| v.eq_ignore_ascii_case(&run_value(&exe)))
}

fn read() -> Option<String> {
    let mut buf = [0u16; 4096];
    let mut bytes = std::mem::size_of_val(&buf) as u32;
    // SAFETY: `buf` and `bytes` outlive the call; `bytes` is the buffer size in bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            KEY,
            NAME,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut bytes),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let len = (bytes as usize / 2).saturating_sub(1).min(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

/// Switches the Run value on or off. `false` when the registry refused.
pub fn set_enabled(on: bool) -> bool {
    if on {
        let Ok(exe) = std::env::current_exe() else {
            return false;
        };
        let value = HSTRING::from(run_value(&exe));
        // SAFETY: `value` lives through the call; the size counts the UTF-16 bytes plus the nul.
        let status = unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                KEY,
                NAME,
                REG_SZ.0,
                Some(value.as_ptr().cast()),
                ((value.len() + 1) * 2) as u32,
            )
        };
        status == ERROR_SUCCESS
    } else {
        // SAFETY: plain call with static strings. A missing value is fine (already off).
        let status = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, KEY, NAME) };
        // Already off counts as done.
        status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_value_is_quoted() {
        let v = run_value(Path::new(r"C:\Program Files\Barista\barista.exe"));
        assert_eq!(v, r#""C:\Program Files\Barista\barista.exe""#);
    }
}
