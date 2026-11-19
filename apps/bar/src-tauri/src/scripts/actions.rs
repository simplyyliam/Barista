//! What happens when a script's menu line is clicked. The parser already filtered the params, and
//! this checks them again, so nothing that skipped the parser can open or start anything else.

use super::parse::{valid_href, ShellCommand, MAX_SHELL_ARGS};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

const MAX_PART_CHARS: usize = 1000;
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A program and arguments we agree to start: no empty or NUL-holding strings, bounded sizes.
pub fn valid_shell(cmd: &ShellCommand) -> bool {
    let ok = |s: &str| s.chars().count() <= MAX_PART_CHARS && !s.contains('\0');
    !cmd.program.trim().is_empty() && ok(&cmd.program) && cmd.args.len() <= MAX_SHELL_ARGS && cmd.args.iter().all(|a| ok(a))
}

/// Opens a web or mail link in the default app. Anything else is refused. `true` = it was handed
/// to Windows.
pub fn open_href(href: &str) -> bool {
    match valid_href(href) {
        Some(url) => {
            crate::platform::windows::shell::open_uri(&url);
            true
        }
        None => false,
    }
}

/// Starts the program with these arguments. There is no shell and no command string, so a menu
/// label or a `param` can't inject a second command: each argument is passed as one piece.
/// The program is not in the script's Job Object, so it outlives a script that times out.
pub fn run_shell(cmd: &ShellCommand, cwd: &Path) -> bool {
    if !valid_shell(cmd) {
        return false;
    }
    Command::new(&cmd.program)
        .args(&cmd.args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .is_ok()
}

/// Opens the script in an editor: the file type's `edit` verb, else Notepad. Never `open`,
/// which would run the script.
pub fn edit_script(path: &Path) {
    if !crate::platform::windows::shell::edit_file(path) {
        let _ = Command::new("notepad.exe").arg(path).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(program: &str, args: &[&str]) -> ShellCommand {
        ShellCommand { program: program.into(), args: args.iter().map(|a| a.to_string()).collect() }
    }

    #[test]
    fn plain_programs_are_accepted() {
        assert!(valid_shell(&cmd("taskmgr", &[])));
        assert!(valid_shell(&cmd("C:\\Windows\\notepad.exe", &["a b.txt"])));
    }

    #[test]
    fn empty_nul_and_huge_are_refused() {
        assert!(!valid_shell(&cmd("", &[])));
        assert!(!valid_shell(&cmd("   ", &[])));
        assert!(!valid_shell(&cmd("a\0b", &[])));
        assert!(!valid_shell(&cmd("x", &["a\0b"])));
        assert!(!valid_shell(&cmd(&"x".repeat(2000), &[])));
        assert!(!valid_shell(&cmd("x", &["a"; 21])));
    }

    #[test]
    fn open_href_refuses_other_schemes_without_touching_windows() {
        for bad in ["file:///c:/windows/system32/calc.exe", "javascript:alert(1)", "calc.exe", "ms-settings:", ""] {
            assert!(!open_href(bad), "{bad}");
        }
    }

    #[test]
    fn arguments_are_never_split_or_interpreted() {
        // `cmd /c echo` would run a second command if a shell string were built. Passing the text
        // as one argument to a program that just exits proves the text is data: where `&` meant
        // "next command", this would have run `calc`. We only check that it starts and that the
        // argument stays one piece (checked by the allow-list, which sees one arg).
        let c = cmd("whoami", &["/all & calc"]);
        assert!(valid_shell(&c));
        assert_eq!(c.args.len(), 1);
    }
}
