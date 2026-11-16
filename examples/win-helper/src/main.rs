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

use std::io::{BufRead, Write};
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

/// Reads command lines until stdin closes, sending each valid one to the worker.
fn read_commands(send: impl Fn(serde_json::Value)) {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.len() > MAX_LINE {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
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
