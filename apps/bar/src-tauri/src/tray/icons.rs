//! Tray icons we already turned into PNG, remembered by a hash of their pixels. Apps flip between
//! a few pictures all day (Discord's unread badge, a sync spinner); the second time we see a
//! picture we skip the conversion. Pure Rust, tested without Windows.

use base64::{engine::general_purpose::STANDARD, Engine};
use std::collections::{HashMap, VecDeque};

/// Pictures kept. Oldest out first.
pub const MAX_ENTRIES: usize = 256;
/// A PNG bigger than this is not worth a data URL for a 28 px pill (rule 3, and memory).
pub const MAX_PNG_BYTES: usize = 64 * 1024;

/// `None` when the PNG is empty or too big to be a tray icon.
pub fn png_to_data_url(png: &[u8]) -> Option<String> {
    if png.is_empty() || png.len() > MAX_PNG_BYTES {
        return None;
    }
    Some(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

#[derive(Default)]
pub struct IconCache {
    // A failed conversion is remembered too (`None`): the same pixels fail the same way.
    map: HashMap<u64, Option<String>>,
    order: VecDeque<u64>,
}

impl IconCache {
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// The data URL for a picture, converting it only the first time.
    pub fn get_or_convert(
        &mut self,
        fingerprint: u64,
        convert: impl FnOnce() -> Option<String>,
    ) -> Option<String> {
        if let Some(hit) = self.map.get(&fingerprint) {
            return hit.clone();
        }
        let value = convert();
        if self.map.len() >= MAX_ENTRIES {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }
        self.map.insert(fingerprint, value.clone());
        self.order.push_back(fingerprint);
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_once_per_picture() {
        let mut cache = IconCache::default();
        let mut calls = 0;
        for _ in 0..3 {
            let url = cache.get_or_convert(7, || {
                calls += 1;
                Some("data".into())
            });
            assert_eq!(url.as_deref(), Some("data"));
        }
        assert_eq!(calls, 1);
        // A different picture converts again.
        cache.get_or_convert(8, || {
            calls += 1;
            None
        });
        assert_eq!(calls, 2);
    }

    #[test]
    fn failures_are_remembered_too() {
        let mut cache = IconCache::default();
        let mut calls = 0;
        for _ in 0..3 {
            let url = cache.get_or_convert(1, || {
                calls += 1;
                None
            });
            assert_eq!(url, None);
        }
        assert_eq!(calls, 1);
    }

    #[test]
    fn stays_bounded_and_drops_the_oldest() {
        let mut cache = IconCache::default();
        for i in 0..(MAX_ENTRIES as u64 + 50) {
            cache.get_or_convert(i, || Some(i.to_string()));
        }
        assert_eq!(cache.len(), MAX_ENTRIES);
        // 0 was the oldest: it is converted again; the newest is still a hit.
        let mut converted = false;
        cache.get_or_convert(0, || {
            converted = true;
            Some("again".into())
        });
        assert!(converted);
        let mut converted = false;
        cache.get_or_convert(MAX_ENTRIES as u64 + 49, || {
            converted = true;
            None
        });
        assert!(!converted);
    }

    #[test]
    fn data_url_limits() {
        assert_eq!(png_to_data_url(&[]), None);
        assert_eq!(png_to_data_url(&vec![0u8; MAX_PNG_BYTES + 1]), None);
        let url = png_to_data_url(&[1, 2, 3]).unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
    }
}
