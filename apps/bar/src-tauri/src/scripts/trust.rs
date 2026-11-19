//! Which scripts the user said yes to (ADR 0034). A script is trusted as (id + SHA-256 of its
//! bytes), so editing the file asks again. Kept in its own `script-trust.json`, not in settings:
//! an imported settings file must never be able to trust a script.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A hand-edited trust file with more entries than this is cut. Nobody has this many scripts.
const MAX_ENTRIES: usize = 1000;

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    /// script id -> hash of the file that was trusted
    #[serde(default)]
    trusted: BTreeMap<String, String>,
}

pub struct TrustStore {
    path: PathBuf,
    file: File,
}

impl TrustStore {
    /// A missing or broken file means "nothing is trusted yet". Never fails: asking again is safe.
    pub fn load(path: PathBuf) -> Self {
        let mut file: File = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        while file.trusted.len() > MAX_ENTRIES {
            file.trusted.pop_last();
        }
        Self { path, file }
    }

    pub fn is_trusted(&self, id: &str, sha256: &str) -> bool {
        !sha256.is_empty() && self.file.trusted.get(id).is_some_and(|h| h == sha256)
    }

    /// Trusts this exact file content. Saves right away; `Err` if the disk refused (the trust then
    /// only lasts until the bar restarts).
    pub fn trust(&mut self, id: &str, sha256: &str) -> std::io::Result<()> {
        if self.file.trusted.len() >= MAX_ENTRIES && !self.file.trusted.contains_key(id) {
            self.file.trusted.pop_last();
        }
        self.file.trusted.insert(id.to_string(), sha256.to_string());
        self.save()
    }

    #[cfg(test)]
    pub fn forget(&mut self, id: &str) {
        self.file.trusted.remove(id);
        let _ = self.save();
    }

    /// Temp file + rename, like settings: a crash can't leave half a file.
    fn save(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.file)?)?;
        std::fs::rename(&tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("barista-trust-{}-{name}.json", std::process::id()))
    }

    #[test]
    fn the_hash_is_sha256_in_hex() {
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn nothing_is_trusted_at_first() {
        let store = TrustStore::load(temp("fresh"));
        assert!(!store.is_trusted("a.ps1", &sha256_hex(b"x")));
    }

    #[test]
    fn trust_survives_a_restart() {
        let path = temp("restart");
        let hash = sha256_hex(b"print('hi')");
        TrustStore::load(path.clone()).trust("a.py", &hash).unwrap();
        assert!(TrustStore::load(path).is_trusted("a.py", &hash));
    }

    #[test]
    fn an_edited_file_is_not_trusted() {
        let mut store = TrustStore::load(temp("edit"));
        store.trust("a.py", &sha256_hex(b"one")).unwrap();
        assert!(!store.is_trusted("a.py", &sha256_hex(b"two")));
    }

    #[test]
    fn trust_is_per_script() {
        let mut store = TrustStore::load(temp("per"));
        let hash = sha256_hex(b"same bytes");
        store.trust("a.py", &hash).unwrap();
        assert!(!store.is_trusted("b.py", &hash));
    }

    #[test]
    fn an_empty_hash_is_never_trusted() {
        let mut store = TrustStore::load(temp("empty"));
        store.trust("a.py", "").unwrap();
        assert!(!store.is_trusted("a.py", ""));
    }

    #[test]
    fn forgetting_asks_again() {
        let mut store = TrustStore::load(temp("forget"));
        let hash = sha256_hex(b"x");
        store.trust("a.py", &hash).unwrap();
        store.forget("a.py");
        assert!(!store.is_trusted("a.py", &hash));
    }

    #[test]
    fn a_broken_file_means_nothing_is_trusted() {
        let path = temp("broken");
        std::fs::write(&path, "{ nope").unwrap();
        assert!(!TrustStore::load(path).is_trusted("a.py", "abc"));
    }

    #[test]
    fn the_file_is_capped() {
        let path = temp("cap");
        let many: BTreeMap<String, String> = (0..MAX_ENTRIES + 50).map(|i| (format!("s{i:05}.py"), "h".into())).collect();
        std::fs::write(&path, serde_json::to_string(&File { trusted: many }).unwrap()).unwrap();
        assert_eq!(TrustStore::load(path).file.trusted.len(), MAX_ENTRIES);
    }
}
