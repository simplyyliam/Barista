//! The signed-in user: display name and picture (for the control center header). Windows has no
//! change event for either, so they are read at start and again when the control center opens.

use super::Service;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// False until the first read finished (or Windows gave no name).
    pub available: bool,
    pub name: String,
    /// The sign-in address, when Windows gives it (a Microsoft account); `None` otherwise.
    pub email: Option<String>,
    /// The picture as a data URL; `None`: draw the initials.
    pub avatar: Option<String>,
}

pub fn service() -> &'static Service<User> {
    static S: OnceLock<Service<User>> = OnceLock::new();
    S.get_or_init(|| Service::new(User::default()))
}

/// The picture bytes as a data URL, only if they really are a JPEG or PNG (never pass on
/// whatever a file contains) and not too big.
pub fn avatar_data_url(bytes: &[u8], max: usize) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    if bytes.is_empty() || bytes.len() > max {
        return None;
    }
    let mime = if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else {
        return None;
    };
    Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}

/// Reads the user on a worker thread (WinRT blocks) and publishes it.
pub fn refresh() {
    #[cfg(windows)]
    {
        use crate::platform::windows::user::{read, MAX_AVATAR_BYTES};
        let spawned = std::thread::Builder::new().name("barista-user".into()).spawn(|| {
            if !super::control_center_active() {
                return;
            }
            if let Some(u) = read() {
                super::publish_gated(service(), User {
                    available: true,
                    name: u.name,
                    email: u.email.map(|e| e.chars().take(120).collect()),
                    avatar: u.avatar.and_then(|b| avatar_data_url(&b, MAX_AVATAR_BYTES)),
                });
            }
        });
        if let Err(e) = spawned {
            eprintln!("barista: could not read the user: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_real_pictures_become_data_urls() {
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 1, 2];
        assert!(avatar_data_url(&jpeg, 100).unwrap().starts_with("data:image/jpeg;base64,"));
        let png = [0x89, b'P', b'N', b'G', 1, 2];
        assert!(avatar_data_url(&png, 100).unwrap().starts_with("data:image/png;base64,"));
        assert_eq!(avatar_data_url(b"<svg onload=x>", 100), None);
        assert_eq!(avatar_data_url(&[], 100), None);
        assert_eq!(avatar_data_url(&jpeg, 3), None);
    }
}
