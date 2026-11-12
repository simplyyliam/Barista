//! The signed-in user's name and picture through `Windows.System.User`, with the plain Win32 name
//! as a fallback when WinRT gives nothing (the picture then stays empty).

use windows::Foundation::IReference;
use windows::Storage::Streams::DataReader;
use windows::System::{KnownUserProperties, User, UserPictureSize};
use windows::Win32::Security::Authentication::Identity::{GetUserNameExW, NameDisplay};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
use windows_core::{Interface, HSTRING, PWSTR};

/// A picture bigger than this is ignored (it would end up in a data URL in every snapshot).
pub const MAX_AVATAR_BYTES: usize = 512 * 1024;

pub struct RawUser {
    pub name: String,
    /// The sign-in address (a Microsoft account's email), if Windows gives it to us.
    pub email: Option<String>,
    /// The picture file as Windows has it (JPEG or PNG).
    pub avatar: Option<Vec<u8>>,
}

fn winrt_name(user: &User) -> Option<String> {
    let value = user
        .GetPropertyAsync(&KnownUserProperties::DisplayName().ok()?)
        .ok()?
        .join()
        .ok()?;
    let name = value.cast::<IReference<HSTRING>>().ok()?.Value().ok()?.to_string();
    (!name.trim().is_empty()).then_some(name)
}

fn winrt_text(user: &User, key: windows_core::Result<HSTRING>) -> Option<String> {
    let value = user.GetPropertyAsync(&key.ok()?).ok()?.join().ok()?;
    let text = value.cast::<IReference<HSTRING>>().ok()?.Value().ok()?.to_string();
    (!text.trim().is_empty()).then_some(text)
}

/// The sign-in address. Local accounts have a plain user name here, so only `a@b` shapes count.
fn winrt_email(user: &User) -> Option<String> {
    [KnownUserProperties::PrincipalName(), KnownUserProperties::AccountName()]
        .into_iter()
        .find_map(|key| winrt_text(user, key).filter(|t| t.contains('@')))
}

fn winrt_avatar(user: &User) -> Option<Vec<u8>> {
    let stream = user
        .GetPictureAsync(UserPictureSize::Size208x208)
        .ok()?
        .join()
        .ok()?
        .OpenReadAsync()
        .ok()?
        .join()
        .ok()?;
    let size = usize::try_from(stream.Size().ok()?).ok()?;
    if size == 0 || size > MAX_AVATAR_BYTES {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(size as u32).ok()?.join().ok()?;
    let mut bytes = vec![0u8; size];
    reader.ReadBytes(&mut bytes).ok()?;
    Some(bytes)
}

fn win32_name() -> Option<String> {
    let mut buf = [0u16; 256];
    let mut len = buf.len() as u32;
    // SAFETY: the buffer and its length match.
    let ok = unsafe { GetUserNameExW(NameDisplay, Some(PWSTR(buf.as_mut_ptr())), &mut len) };
    if ok {
        let name = String::from_utf16_lossy(&buf[..len as usize]);
        if !name.trim().is_empty() {
            return Some(name);
        }
    }
    std::env::var("USERNAME").ok().filter(|n| !n.is_empty())
}

/// Reads the user. Call from a thread that may block (it waits on WinRT).
pub fn read() -> Option<RawUser> {
    // SAFETY: plain COM init for this thread; WinRT needs an apartment.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();
    }
    let users = User::FindAllAsync().ok().and_then(|op| op.join().ok());
    let user = users.and_then(|list| list.into_iter().next());
    let name = user.as_ref().and_then(winrt_name).or_else(win32_name)?;
    let email = user.as_ref().and_then(winrt_email);
    let avatar = user.as_ref().and_then(winrt_avatar);
    Some(RawUser { name, email, avatar })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "needs a signed-in desktop session"]
    fn reads_the_signed_in_user() {
        let u = read().expect("a user");
        println!("name={:?} avatar={:?} bytes", u.name, u.avatar.as_ref().map(|a| a.len()));
        assert!(!u.name.is_empty());
    }
}
