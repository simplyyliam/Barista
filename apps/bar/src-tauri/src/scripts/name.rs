//! `name.INTERVAL.ext` into an id, an interval and an interpreter (ADR 0035). Pure.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpreter {
    PowerShell,
    Python,
    Node,
}

impl Interpreter {
    pub fn from_ext(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "ps1" => Some(Self::PowerShell),
            "py" => Some(Self::Python),
            "js" | "mjs" => Some(Self::Node),
            _ => None,
        }
    }
    /// For the trust dialog and error messages.
    pub fn label(self) -> &'static str {
        match self {
            Self::PowerShell => "PowerShell",
            Self::Python => "Python",
            Self::Node => "Node.js",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptName {
    /// Stable key: lowercase `[a-z0-9-_.]`, interval removed, extension kept (`clock.mjs`).
    pub id: String,
    /// The name as the user wrote it, without interval and extension (`Clock`).
    pub title: String,
    /// From the file name; `None` = use the settings.
    pub interval_secs: Option<u32>,
    pub interpreter: Interpreter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameError {
    /// Not a script type we run (a `.txt`, a folder's `desktop.ini`): ignore it silently.
    NotAScript,
    /// It looks like an interval but isn't one (`500ms`, `0s`, `5x`).
    BadInterval(String),
}

/// 30 days: more is almost certainly a typo and `u32` seconds must not overflow.
const MAX_INTERVAL_SECS: u64 = 30 * 86_400;

/// `10s`, `5m`, `2h`, `1d`. Minimum 1 s.
fn interval_of(segment: &str) -> Option<Result<u32, ()>> {
    let digits = segment.chars().take_while(|c| c.is_ascii_digit()).count();
    let (number, unit) = segment.split_at(digits);
    // It only looks like an interval if it is digits then letters.
    if digits == 0 || unit.is_empty() || !unit.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let per = match unit.to_ascii_lowercase().as_str() {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86_400,
        _ => return Some(Err(())),
    };
    // Too many digits for a u64 is just "very long": capped below.
    let secs = Some(number.parse::<u64>().unwrap_or(u64::MAX).saturating_mul(per));
    match secs {
        Some(s) if s >= 1 => Some(Ok(s.min(MAX_INTERVAL_SECS) as u32)),
        _ => Some(Err(())),
    }
}

pub fn parse_file_name(file_name: &str) -> Result<ScriptName, NameError> {
    let (stem, ext) = file_name.rsplit_once('.').ok_or(NameError::NotAScript)?;
    let interpreter = Interpreter::from_ext(ext).ok_or(NameError::NotAScript)?;
    if stem.is_empty() {
        return Err(NameError::NotAScript);
    }
    let (base, interval_secs) = match stem.rsplit_once('.') {
        // The interval needs a name before it: `10s.ps1` is a script called "10s".
        Some((base, segment)) if !base.is_empty() => match interval_of(segment) {
            Some(Ok(secs)) => (base, Some(secs)),
            Some(Err(())) => return Err(NameError::BadInterval(segment.to_string())),
            None => (stem, None),
        },
        _ => (stem, None),
    };
    Ok(ScriptName {
        id: format!("{}.{}", map_id(base), ext.to_ascii_lowercase()),
        title: base.to_string(),
        interval_secs,
        interpreter,
    })
}

/// Lowercase, anything outside `[a-z0-9-_.]` becomes `_`, at most 64 characters.
fn map_id(base: &str) -> String {
    base.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.') { c } else { '_' })
        .take(64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(name: &str) -> ScriptName {
        parse_file_name(name).unwrap()
    }

    #[test]
    fn name_interval_and_extension() {
        let n = ok("cpu.10s.ps1");
        assert_eq!((n.id.as_str(), n.title.as_str(), n.interval_secs), ("cpu.ps1", "cpu", Some(10)));
        assert_eq!(n.interpreter, Interpreter::PowerShell);
    }

    #[test]
    fn all_units() {
        assert_eq!(ok("a.1s.py").interval_secs, Some(1));
        assert_eq!(ok("a.5m.py").interval_secs, Some(300));
        assert_eq!(ok("a.2h.py").interval_secs, Some(7200));
        assert_eq!(ok("a.1d.py").interval_secs, Some(86_400));
    }

    #[test]
    fn the_interval_is_optional() {
        let n = ok("hello.py");
        assert_eq!((n.id.as_str(), n.interval_secs), ("hello.py", None));
    }

    #[test]
    fn changing_the_interval_keeps_the_id() {
        assert_eq!(ok("clock.10s.mjs").id, ok("clock.2s.mjs").id);
        assert_eq!(ok("clock.10s.mjs").id, "clock.mjs");
    }

    #[test]
    fn interpreters_by_extension() {
        assert_eq!(ok("a.js").interpreter, Interpreter::Node);
        assert_eq!(ok("a.MJS").interpreter, Interpreter::Node);
        assert_eq!(ok("a.PS1").interpreter, Interpreter::PowerShell);
        assert_eq!(ok("a.py").interpreter, Interpreter::Python);
    }

    #[test]
    fn other_files_are_not_scripts() {
        for n in ["notes.txt", "desktop.ini", "run", ".ps1", "a.sh", "a.bat", "a.ps1.bak"] {
            assert_eq!(parse_file_name(n), Err(NameError::NotAScript), "{n}");
        }
    }

    #[test]
    fn milliseconds_and_zero_are_rejected() {
        for n in ["a.500ms.ps1", "a.0s.ps1", "a.0m.py", "a.5x.py", "a.10.py.js"] {
            // "10" alone (digits only) is part of the name, not an interval.
            if n == "a.10.py.js" {
                assert!(parse_file_name(n).is_ok());
                continue;
            }
            assert!(matches!(parse_file_name(n), Err(NameError::BadInterval(_))), "{n}");
        }
    }

    #[test]
    fn a_name_that_is_only_an_interval_is_a_name() {
        let n = ok("10s.ps1");
        assert_eq!((n.id.as_str(), n.interval_secs), ("10s.ps1", None));
    }

    #[test]
    fn huge_intervals_are_capped_not_overflowed() {
        assert_eq!(ok("a.99999999999999999999d.py").interval_secs, Some(30 * 86_400));
        assert_eq!(ok("a.400d.py").interval_secs, Some(30 * 86_400));
        assert_eq!(ok("a.18446744073709551615d.py").interval_secs, Some(30 * 86_400));
    }

    #[test]
    fn ids_are_mapped_to_a_safe_alphabet() {
        let n = ok("My Script (2)!.5s.ps1");
        assert_eq!(n.id, "my_script__2__.ps1");
        assert_eq!(n.title, "My Script (2)!");
        assert!(n.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-_.".contains(c)));
    }

    #[test]
    fn ids_never_contain_a_slash_or_path_bits() {
        let n = ok("..\\..\\evil.1s.ps1");
        assert!(!n.id.contains('\\') && !n.id.contains('/'));
    }

    #[test]
    fn ids_are_capped_at_64_plus_extension() {
        let n = ok(&format!("{}.py", "a".repeat(300)));
        assert_eq!(n.id.len(), 64 + 3);
    }
}
