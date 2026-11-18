//! One small program that gives the first-party items (mic mute, now playing) the Windows APIs
//! Node cannot reach. It speaks JSON lines: events go out on stdout, commands come in on stdin,
//! and it exits when stdin closes (so it never outlives the app that started it). ADR 0030.
//!
//! ```text
//! win-helper --mic     out: {"type":"mic","present":true,"muted":false,"device":"..."}
//!                      in:  {"cmd":"toggle"} | {"cmd":"setMuted","muted":true}
//! win-helper --media   out: {"type":"media","active":true,"playing":true,"title":"..","artist":"..",
//!                            "app":"..","canPrevious":true,"canNext":true}
//!                      in:  {"cmd":"playPause"} | {"cmd":"next"} | {"cmd":"previous"}
//! ```

#[cfg(windows)]
mod media;
#[cfg(windows)]
mod mic;

use std::io::Write;
use std::sync::mpsc::Sender;

/// A command line longer than this is not ours (rule 3: never trust what arrives on a pipe).
const MAX_LINE: usize = 1024;

/// Prints one JSON line, only when it differs from the last one printed.
pub struct Out {
    last: std::sync::Mutex<String>,
}

impl Default for Out {
    fn default() -> Self {
        Self { last: std::sync::Mutex::new(String::new()) }
    }
}

impl Out {

    pub fn emit(&self, value: serde_json::Value) {
        let line = value.to_string();
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        if *last == line {
            return;
        }
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        // A closed stdout means the parent is gone: stop.
        if writeln!(lock, "{line}").and_then(|_| lock.flush()).is_err() {
            std::process::exit(0);
        }
        *last = line;
    }
}

/// Reads command lines until stdin closes, sending each valid one to the worker. A line is never
/// held in memory past `MAX_LINE` bytes: the rest of an oversized line is thrown away as it arrives.
fn read_commands(send: impl Fn(serde_json::Value)) {
    use std::io::{BufRead, Read};
    let mut stdin = std::io::stdin().lock();
    let mut line = Vec::new();
    loop {
        line.clear();
        let Ok(read) = (&mut stdin).take(MAX_LINE as u64 + 1).read_until(b'\n', &mut line) else { break };
        if read == 0 {
            break;
        }
        if !line.ends_with(b"\n") && line.len() > MAX_LINE {
            // Too long: drop what is left of this line without keeping it.
            loop {
                let (skip, found) = match stdin.fill_buf() {
                    Ok([]) | Err(_) => return,
                    Ok(buf) => match buf.iter().position(|&b| b == b'\n') {
                        Some(i) => (i + 1, true),
                        None => (buf.len(), false),
                    },
                };
                stdin.consume(skip);
                if found {
                    break;
                }
            }
            continue;
        }
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) {
            send(value);
        }
    }
}

#[cfg(windows)]
fn main() {
    let mode = std::env::args().nth(1).unwrap_or_default();
    let out = std::sync::Arc::new(Out::default());
    match mode.as_str() {
        "--mic" => {
            let tx = mic::start(out);
            read_commands(|v| send(&tx, v));
        }
        "--media" => {
            let tx = media::start(out);
            read_commands(|v| send(&tx, v));
        }
        _ => {
            eprintln!("usage: win-helper --mic | --media");
            std::process::exit(2);
        }
    }
}

#[cfg(windows)]
fn send(tx: &Sender<serde_json::Value>, v: serde_json::Value) {
    let _ = tx.send(v);
}

#[cfg(not(windows))]
fn main() {
    eprintln!("win-helper only runs on Windows");
    std::process::exit(2);
}
