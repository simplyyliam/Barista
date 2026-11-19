//! Runs one script once and hands back its stdout (ADR 0034). Everything the script does is
//! bounded: time, output size and the process tree (a Job Object, see `platform/windows/job.rs`).
//! Dropping the returned future kills the tree, so cancelling a script is just aborting its task.

use super::name::Interpreter;
use super::parse::MAX_OUTPUT_BYTES;
use crate::platform::windows::job::{self, Job};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

/// How much of stderr we keep for "Show error".
const MAX_STDERR_BYTES: usize = 4096;
/// How long to wait for a killed script's pipes to close before giving up on them.
const PIPE_GRACE: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// "Python not found". The interpreter isn't installed or isn't on PATH.
    NoInterpreter(String),
    /// The process could not be started or put in its job.
    Start(String),
    Timeout(Duration),
    TooLarge,
    /// A non-zero exit. `stderr` is the first 4 KB, lossy.
    Exit { code: Option<i32>, stderr: String },
}

impl RunError {
    /// One line for the item's error menu.
    pub fn message(&self) -> String {
        match self {
            Self::NoInterpreter(m) | Self::Start(m) => m.clone(),
            Self::Timeout(d) => format!("The script took longer than {} s and was stopped", d.as_secs().max(1)),
            Self::TooLarge => "The script printed more than 64 KB and was stopped".into(),
            Self::Exit { code: Some(c), .. } => format!("The script exited with code {c}"),
            Self::Exit { code: None, .. } => "The script was stopped by Windows".into(),
        }
    }

    /// Extra lines (stderr), for the "Show error" submenu.
    pub fn detail(&self) -> &str {
        match self {
            Self::Exit { stderr, .. } => stderr,
            _ => "",
        }
    }
}

pub struct RunSpec {
    pub path: PathBuf,
    pub interpreter: Interpreter,
    pub timeout: Duration,
    /// The scripts folder, so relative paths in a script point next to it.
    pub cwd: PathBuf,
}

/// Looks `name.exe` up in every PATH folder. `skip_store_stub` ignores the empty `python.exe` that
/// Windows ships in `WindowsApps` (it opens the Microsoft Store instead of running anything).
fn find_on_path(name: &str, skip_store_stub: bool) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .filter(|dir| !(skip_store_stub && dir.to_string_lossy().to_ascii_lowercase().contains("windowsapps")))
        .map(|dir| dir.join(format!("{name}.exe")))
        .find(|candidate| candidate.is_file())
}

/// The program and its leading arguments (the script path comes last).
pub fn command_for(interpreter: Interpreter) -> Result<(PathBuf, Vec<OsString>), RunError> {
    let missing = |what: &str| RunError::NoInterpreter(format!("{what} not found"));
    match interpreter {
        Interpreter::PowerShell => {
            let exe = find_on_path("pwsh", false)
                .or_else(|| {
                    let root = std::env::var_os("SystemRoot")?;
                    let p = Path::new(&root).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
                    p.is_file().then_some(p)
                })
                .ok_or_else(|| missing("PowerShell"))?;
            // Bypass: the default policy blocks every .ps1. The trust prompt is the gate here.
            let args = ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"];
            Ok((exe, args.iter().map(OsString::from).collect()))
        }
        Interpreter::Python => {
            let exe = find_on_path("py", false)
                .or_else(|| find_on_path("python", true))
                .ok_or_else(|| missing("Python"))?;
            Ok((exe, vec![]))
        }
        Interpreter::Node => Ok((find_on_path("node", false).ok_or_else(|| missing("Node.js"))?, vec![])),
    }
}

/// Reads up to `cap` bytes, then stops. `true` = there was more.
async fn read_capped(mut from: impl AsyncRead + Unpin, cap: usize) -> (Vec<u8>, bool) {
    let mut out = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match from.read(&mut chunk).await {
            Ok(0) | Err(_) => return (out, false),
            Ok(n) => {
                if out.len() + n > cap {
                    out.extend_from_slice(&chunk[..cap - out.len()]);
                    return (out, true);
                }
                out.extend_from_slice(&chunk[..n]);
            }
        }
    }
}

/// Keeps the first `cap` bytes and throws the rest away. It must keep reading: a script that
/// fills the pipe's buffer would otherwise block on its next write to stderr, forever.
async fn read_head(mut from: impl AsyncRead + Unpin, cap: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match from.read(&mut chunk).await {
            Ok(0) | Err(_) => return out,
            Ok(n) => {
                let room = cap.saturating_sub(out.len());
                out.extend_from_slice(&chunk[..n.min(room)]);
            }
        }
    }
}

pub async fn run(spec: RunSpec) -> Result<Vec<u8>, RunError> {
    let (exe, lead_args) = command_for(spec.interpreter)?;
    let start = |e: String| RunError::Start(format!("Could not start the script: {e}"));

    let mut command = Command::new(exe);
    command
        .args(lead_args)
        .arg(&spec.path)
        .current_dir(&spec.cwd)
        .env("BARISTA", "1")
        .env("BARISTA_VERSION", env!("CARGO_PKG_VERSION"))
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Backup for the job: if our future is dropped before the job exists, still kill the child.
        .kill_on_drop(true)
        .creation_flags(job::CREATION_FLAGS);

    let job = Job::new().map_err(start)?;
    let mut child = command.spawn().map_err(|e| start(e.to_string()))?;
    // Frozen at this point. Into the job first, then let it run (see job.rs).
    let (Some(pid), Some(handle)) = (child.id(), child.raw_handle()) else {
        return Err(start("the process ended at once".into()));
    };
    job.add(handle).map_err(start)?;
    job::resume(pid).map_err(start)?;

    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        return Err(start("no output pipes".into()));
    };
    let mut out_task = tokio::spawn(read_capped(stdout, MAX_OUTPUT_BYTES));
    let err_task = tokio::spawn(read_head(stderr, MAX_STDERR_BYTES));

    let body = async {
        // Whichever comes first: stdout ends (or hits the cap), or the process exits.
        let (stdout, exceeded, status) = tokio::select! {
            res = &mut out_task => {
                let (out, exceeded) = res.unwrap_or_default();
                if exceeded {
                    return Err(RunError::TooLarge);
                }
                (out, false, child.wait().await.ok())
            }
            status = child.wait() => {
                // It exited. Kill anything it left behind (a background child could hold the pipe
                // open forever), then collect what is already in the pipe.
                job.terminate();
                let (out, exceeded) = tokio::time::timeout(PIPE_GRACE, &mut out_task)
                    .await
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or_default();
                (out, exceeded, status.ok())
            }
        };
        if exceeded {
            return Err(RunError::TooLarge);
        }
        Ok((stdout, status))
    };

    let result = tokio::time::timeout(spec.timeout, body).await;
    // Whatever happened, nothing of this run stays alive.
    job.terminate();
    let stderr_bytes = tokio::time::timeout(PIPE_GRACE, err_task).await.ok().and_then(Result::ok).unwrap_or_default();
    out_task.abort();

    match result {
        Err(_) => Err(RunError::Timeout(spec.timeout)),
        Ok(Err(e)) => Err(e),
        Ok(Ok((stdout, status))) => match status {
            Some(s) if s.success() => Ok(stdout),
            Some(s) => Err(RunError::Exit {
                code: s.code(),
                stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
            }),
            None => Err(RunError::Exit { code: None, stderr: String::new() }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A script file in a fresh temp folder. Tests that need node skip themselves without it.
    fn node_script(code: &str) -> Option<RunSpec> {
        static N: AtomicU32 = AtomicU32::new(0);
        find_on_path("node", false)?;
        let dir = std::env::temp_dir().join(format!("barista-run-test-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.mjs");
        std::fs::write(&path, code).unwrap();
        Some(RunSpec { path, interpreter: Interpreter::Node, timeout: Duration::from_secs(5), cwd: dir })
    }

    fn alive(pid: u32) -> bool {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    }

    #[tokio::test]
    async fn prints_its_output() {
        let Some(spec) = node_script("console.log('hello'); console.log('---'); console.log('menu');") else { return };
        let out = run(spec).await.unwrap();
        assert_eq!(String::from_utf8_lossy(&out).replace("\r\n", "\n"), "hello\n---\nmenu\n");
    }

    #[tokio::test]
    async fn a_non_zero_exit_keeps_stderr() {
        let Some(spec) = node_script("console.error('boom'); process.exit(1);") else { return };
        match run(spec).await {
            Err(RunError::Exit { code: Some(1), stderr }) => assert!(stderr.contains("boom")),
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn a_script_that_sleeps_forever_is_stopped_at_the_timeout() {
        let Some(mut spec) = node_script("setInterval(() => {}, 1000);") else { return };
        spec.timeout = Duration::from_millis(700);
        let started = std::time::Instant::now();
        assert_eq!(run(spec).await, Err(RunError::Timeout(Duration::from_millis(700))));
        assert!(started.elapsed() < Duration::from_secs(4));
    }

    #[tokio::test]
    async fn ten_megabytes_of_output_is_cut_off() {
        let Some(spec) = node_script("const b = 'x'.repeat(1024 * 1024); for (let i = 0; i < 10; i++) process.stdout.write(b);") else { return };
        assert_eq!(run(spec).await, Err(RunError::TooLarge));
    }

    #[tokio::test]
    async fn binary_output_comes_back_as_bytes_for_the_parser_to_reject() {
        let Some(spec) = node_script("process.stdout.write(Buffer.from([0, 255, 254, 0, 1]));") else { return };
        let out = run(spec).await.unwrap();
        assert!(super::super::parse::parse(&out).is_err());
    }

    #[tokio::test]
    async fn noisy_stderr_does_not_block_the_script() {
        let Some(spec) = node_script("process.stderr.write('e'.repeat(2 * 1024 * 1024)); console.log('done');") else { return };
        let out = run(spec).await.unwrap();
        assert!(String::from_utf8_lossy(&out).contains("done"));
    }

    #[tokio::test]
    async fn the_whole_process_tree_dies_on_timeout() {
        // The script starts a grandchild that would live for a minute and prints its pid.
        let code = r#"
            import { spawn } from 'node:child_process';
            const c = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], { stdio: 'ignore' });
            console.log(c.pid);
            setInterval(() => {}, 1000);
        "#;
        let Some(mut spec) = node_script(code) else { return };
        spec.timeout = Duration::from_millis(1500);
        let dir = spec.cwd.clone();
        // The timeout drops the output, so ask the grandchild's pid another way: the script also
        // writes it to a file.
        std::fs::write(
            &spec.path,
            r#"
            import { spawn } from 'node:child_process';
            import { writeFileSync } from 'node:fs';
            const c = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], { stdio: 'ignore' });
            writeFileSync('child.pid', String(c.pid));
            setInterval(() => {}, 1000);
            "#,
        )
        .unwrap();
        assert!(matches!(run(spec).await, Err(RunError::Timeout(_))));
        let pid: u32 = std::fs::read_to_string(dir.join("child.pid")).unwrap().trim().parse().unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(!alive(pid), "the grandchild {pid} is still running");
    }

    #[tokio::test]
    async fn dropping_the_future_kills_the_tree() {
        let Some(mut spec) = node_script(
            r#"
            import { spawn } from 'node:child_process';
            import { writeFileSync } from 'node:fs';
            const c = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], { stdio: 'ignore' });
            writeFileSync('child.pid', String(c.pid));
            setInterval(() => {}, 1000);
            "#,
        ) else { return };
        spec.timeout = Duration::from_secs(30);
        let dir = spec.cwd.clone();
        let task = tokio::spawn(run(spec));
        for _ in 0..50 {
            if dir.join("child.pid").exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
        let pid: u32 = std::fs::read_to_string(dir.join("child.pid")).unwrap().trim().parse().unwrap();
        task.abort();
        let _ = task.await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(!alive(pid), "the grandchild {pid} survived the cancel");
    }

    #[tokio::test]
    async fn a_background_child_does_not_hold_the_run_open() {
        // The script prints and exits, leaving a child that keeps stdout open.
        let Some(spec) = node_script(
            r#"
            import { spawn } from 'node:child_process';
            // Detached and unref'd, so node itself exits and only the child keeps stdout open.
            const c = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'], { stdio: 'inherit', detached: true });
            c.unref();
            console.log('label');
            "#,
        ) else { return };
        let started = std::time::Instant::now();
        let out = run(spec).await.unwrap();
        assert!(String::from_utf8_lossy(&out).contains("label"));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn a_missing_interpreter_is_a_clear_error() {
        // Only checks the message format; the real lookup depends on the machine.
        let e = RunError::NoInterpreter("Python not found".into());
        assert_eq!(e.message(), "Python not found");
    }

    #[test]
    fn error_messages_are_one_line() {
        for e in [
            RunError::Timeout(Duration::from_secs(10)),
            RunError::TooLarge,
            RunError::Exit { code: Some(2), stderr: "x\ny".into() },
        ] {
            assert!(!e.message().contains('\n'));
        }
    }
}
