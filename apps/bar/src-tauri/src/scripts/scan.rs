//! Reads the scripts folder and does the timing math. No windows, no tasks: plain functions, so the
//! rescan rules (add, remove, rename, edit) can be tested without a running bar.

use super::name::{parse_file_name, NameError, ScriptName};
use super::trust::sha256_hex;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// More files than this in the folder are ignored (a script item is a bar item, not a database).
pub const MAX_SCRIPTS: usize = 100;
/// Bigger files are not read or hashed. Scripts are small.
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;
pub const MAX_TIMEOUT: Duration = Duration::from_secs(10);
/// Failures back off to this many seconds (unless the script's own interval is longer).
const BACKOFF_CAP_SECS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoundKind {
    Script(ScriptName),
    /// Two files map to the same id (`a.5s.py` and `a.10s.py`). The first by file name wins.
    Duplicate(ScriptName),
    /// Not usable; the text says why (shown in settings).
    Invalid(String),
    /// Could not be read right now (an editor has it locked). The manager leaves its state alone.
    Unreadable(ScriptName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub file_name: String,
    pub path: PathBuf,
    pub size: u64,
    /// Hex SHA-256 of the file now; empty when the file was not read.
    pub sha256: String,
    /// The first lines, for the trust dialog.
    pub preview: Vec<String>,
    /// The preview is not the whole file.
    pub preview_cut: bool,
    pub kind: FoundKind,
}

/// Every script-looking file directly in `dir`, sorted by file name (so the duplicate winner is
/// stable). Folders and other file types are skipped.
pub fn scan(dir: &Path) -> Vec<Found> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut names: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            // `metadata` follows links; a folder named `x.ps1` is not a script.
            let meta = std::fs::metadata(e.path()).ok()?;
            meta.is_file().then(|| (name, e.path()))
        })
        .collect();
    names.sort_by_key(|(n, _)| n.to_lowercase());

    let mut out: Vec<Found> = Vec::new();
    for (file_name, path) in names {
        let parsed = match parse_file_name(&file_name) {
            Err(NameError::NotAScript) => continue,
            Err(NameError::BadInterval(seg)) => {
                out.push(Found {
                    file_name,
                    path,
                    size: 0,
                    sha256: String::new(),
                    preview: vec![], preview_cut: false,
                    kind: FoundKind::Invalid(format!("\"{seg}\" is not an interval (use 10s, 5m, 2h or 1d, at least 1s)")),
                });
                continue;
            }
            Ok(name) => name,
        };
        if out.iter().filter(|f| matches!(f.kind, FoundKind::Script(_))).count() >= MAX_SCRIPTS {
            break;
        }
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size > MAX_FILE_BYTES {
            out.push(Found { file_name, path, size, sha256: String::new(), preview: vec![], preview_cut: false, kind: FoundKind::Invalid("The file is larger than 1 MB".into()) });
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            out.push(Found { file_name, path, size, sha256: String::new(), preview: vec![], preview_cut: false, kind: FoundKind::Unreadable(parsed) });
            continue;
        };
        let (shown, cut) = preview(&bytes);
        let duplicate = out.iter().any(|f| matches!(&f.kind, FoundKind::Script(n) if n.id == parsed.id));
        out.push(Found {
            file_name,
            path,
            size: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
            preview: shown,
            preview_cut: cut,
            kind: if duplicate { FoundKind::Duplicate(parsed) } else { FoundKind::Script(parsed) },
        });
    }
    out
}

/// File name first, then the script's own setting, then the default. At least 1 s.
pub fn effective_interval(from_name: Option<u32>, from_settings: Option<u32>, default: u32) -> u32 {
    from_name.or(from_settings).unwrap_or(default).max(1)
}

/// 10 s, but never longer than the interval: a run must be over before the next one is due.
pub fn timeout_for(interval_secs: u32) -> Duration {
    MAX_TIMEOUT.min(Duration::from_secs(interval_secs.max(1) as u64))
}

/// How long to wait after a run: the interval, doubled after each failure in a row, up to 5 min.
/// A script whose own interval is longer than that is never made faster by a failure.
pub fn next_delay(interval_secs: u32, failures: u32) -> Duration {
    let base = interval_secs.max(1) as u64;
    let cap = BACKOFF_CAP_SECS.max(base);
    let factor = 1u64 << failures.min(20);
    Duration::from_secs(base.saturating_mul(factor).min(cap))
}

/// The first lines of a script for the trust dialog: control characters stripped, lines and
/// total cut short. Lossy on purpose: this is only to look at. The flag says the dialog does not
/// show everything (more lines, a long line cut, or a big file), so it can say so.
pub fn preview(bytes: &[u8]) -> (Vec<String>, bool) {
    const LINES: usize = 12;
    const LINE_CHARS: usize = 100;
    const TOTAL_CHARS: usize = 1500;
    const HEAD_BYTES: usize = 8 * 1024;
    let head = &bytes[..bytes.len().min(HEAD_BYTES)];
    let text = String::from_utf8_lossy(head);
    let mut cut = bytes.len() > HEAD_BYTES;
    let mut total = 0;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i >= LINES {
            cut = true;
            break;
        }
        let clean: String = line.chars().map(|c| if c == '\t' { ' ' } else { c }).filter(|c| !c.is_control()).collect();
        let shown: String = clean.chars().take(LINE_CHARS).collect();
        cut |= clean.chars().count() > LINE_CHARS;
        total += shown.chars().count();
        if total > TOTAL_CHARS {
            cut = true;
            break;
        }
        out.push(shown);
    }
    (out, cut)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("barista-scan-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }
    fn write(d: &Path, name: &str, text: &str) {
        std::fs::write(d.join(name), text).unwrap();
    }
    fn ids(found: &[Found]) -> Vec<String> {
        found
            .iter()
            .filter_map(|f| match &f.kind {
                FoundKind::Script(n) => Some(n.id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn finds_scripts_and_skips_everything_else() {
        let d = dir("basic");
        write(&d, "cpu.10s.ps1", "x");
        write(&d, "hello.30s.py", "x");
        write(&d, "notes.txt", "x");
        write(&d, "desktop.ini", "x");
        std::fs::create_dir(d.join("sub.5s.ps1")).unwrap();
        assert_eq!(ids(&scan(&d)), vec!["cpu.ps1", "hello.py"]);
    }

    #[test]
    fn a_missing_folder_is_empty() {
        assert!(scan(&std::env::temp_dir().join("barista-no-such-folder-xyz")).is_empty());
    }

    #[test]
    fn the_hash_follows_the_content() {
        let d = dir("hash");
        write(&d, "a.5s.py", "one");
        let first = scan(&d)[0].sha256.clone();
        write(&d, "a.5s.py", "two");
        let second = scan(&d)[0].sha256.clone();
        assert_ne!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn renaming_only_the_interval_keeps_id_and_hash() {
        let d = dir("rename");
        write(&d, "clock.10s.mjs", "same");
        let before = scan(&d);
        std::fs::rename(d.join("clock.10s.mjs"), d.join("clock.2s.mjs")).unwrap();
        let after = scan(&d);
        assert_eq!(ids(&before), ids(&after));
        assert_eq!(before[0].sha256, after[0].sha256);
        match &after[0].kind {
            FoundKind::Script(n) => assert_eq!(n.interval_secs, Some(2)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_deleted_file_is_gone() {
        let d = dir("delete");
        write(&d, "a.5s.py", "x");
        std::fs::remove_file(d.join("a.5s.py")).unwrap();
        assert!(scan(&d).is_empty());
    }

    #[test]
    fn a_bad_interval_is_listed_as_invalid() {
        let d = dir("bad");
        write(&d, "a.500ms.ps1", "x");
        let found = scan(&d);
        assert!(matches!(&found[0].kind, FoundKind::Invalid(m) if m.contains("500ms")));
    }

    #[test]
    fn the_second_file_with_the_same_id_is_a_duplicate() {
        let d = dir("dup");
        write(&d, "a.10s.py", "x");
        write(&d, "a.5s.py", "x");
        let found = scan(&d);
        assert_eq!(found.len(), 2);
        // "a.10s.py" sorts before "a.5s.py", so it wins.
        assert!(matches!(found[0].kind, FoundKind::Script(_)));
        assert!(matches!(found[1].kind, FoundKind::Duplicate(_)));
    }

    #[test]
    fn big_files_are_not_read() {
        let d = dir("big");
        std::fs::write(d.join("a.5s.py"), vec![b'x'; MAX_FILE_BYTES as usize + 1]).unwrap();
        let found = scan(&d);
        assert!(matches!(found[0].kind, FoundKind::Invalid(_)));
        assert!(found[0].sha256.is_empty());
    }

    #[test]
    fn the_interval_order_is_name_then_settings_then_default() {
        assert_eq!(effective_interval(Some(5), Some(30), 60), 5);
        assert_eq!(effective_interval(None, Some(30), 60), 30);
        assert_eq!(effective_interval(None, None, 60), 60);
        assert_eq!(effective_interval(Some(0), None, 60), 1);
    }

    #[test]
    fn the_timeout_is_10_s_but_never_past_the_interval() {
        assert_eq!(timeout_for(60), Duration::from_secs(10));
        assert_eq!(timeout_for(10), Duration::from_secs(10));
        assert_eq!(timeout_for(2), Duration::from_secs(2));
        assert_eq!(timeout_for(1), Duration::from_secs(1));
    }

    #[test]
    fn failures_double_the_wait_up_to_five_minutes() {
        let secs = |f| next_delay(10, f).as_secs();
        assert_eq!([secs(0), secs(1), secs(2), secs(3), secs(4), secs(5), secs(40)], [10, 20, 40, 80, 160, 300, 300]);
    }

    #[test]
    fn a_long_interval_is_not_shortened_by_a_failure() {
        assert_eq!(next_delay(3600, 0).as_secs(), 3600);
        assert_eq!(next_delay(3600, 3).as_secs(), 3600);
    }

    #[test]
    fn the_preview_is_short_and_clean() {
        let text = format!("line1\n\u{1b}[31mred\u{7}\n{}\n", "x".repeat(500));
        let (lines, cut) = preview(text.as_bytes());
        assert!(cut, "a 500 character line is cut");
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "[31mred");
        assert_eq!(lines[2].chars().count(), 100);
        let many = "a\n".repeat(100);
        let (lines, cut) = preview(many.as_bytes());
        assert_eq!((lines.len(), cut), (12, true));
        assert!(preview(&[0xff, 0xfe, 0x00]).0.len() <= 12);
        assert_eq!(preview(b"one
two
"), (vec!["one".to_string(), "two".to_string()], false));
    }
}
