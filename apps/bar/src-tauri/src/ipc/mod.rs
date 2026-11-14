//! The named-pipe server (`\\.\pipe\barista`): accepts clients and gives each one its own
//! session task (see `session.rs`). Message types come from `crates/protocol`; every rule on
//! what a client may send lives in `validate.rs`.

mod frame;
mod session;
mod validate;

use crate::platform::windows::pipe;
use crate::registry::Registry;
use session::{Heartbeat, SessionConfig};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{watch, Semaphore};

/// Most clients connected at once. Each one costs a task and a few buffers, so this bounds
/// what a misbehaving program can use up.
const MAX_CONNECTIONS: usize = 64;

/// Turns true when the bar quits. Every session watches it and tells its client.
fn quit_signal() -> &'static watch::Sender<bool> {
    static SIGNAL: OnceLock<watch::Sender<bool>> = OnceLock::new();
    SIGNAL.get_or_init(|| watch::channel(false).0)
}

/// Sessions still running (including the moment they flush their last message).
static LIVE_SESSIONS: AtomicUsize = AtomicUsize::new(0);

/// Starts listening in the background. A failure is logged and never stops the bar: it just
/// runs without clients (for example when another bar already owns the pipe).
pub fn start(registry: Arc<Registry>) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = serve(registry).await {
            eprintln!("barista: the client pipe is not available: {error}");
        }
    });
}

/// The bar is quitting: stop accepting, tell every client (`shutdown { quit }`) and give the
/// messages a moment to leave. Blocks at most `wait`; the process is about to end anyway.
pub fn shutdown(wait: Duration) {
    quit_signal().send_replace(true);
    let deadline = Instant::now() + wait;
    while LIVE_SESSIONS.load(Ordering::SeqCst) > 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
}

async fn serve(registry: Arc<Registry>) -> std::io::Result<()> {
    let bar_version = env!("CARGO_PKG_VERSION").to_string();
    let permits = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    let mut quit = quit_signal().subscribe();
    // The first instance claims the name; a second bar fails here instead of sharing it.
    let mut server = pipe::create_server(true)?;
    loop {
        let connecting = tokio::select! {
            result = server.connect() => result,
            // Quitting: drop the pipe so that clients retrying meanwhile do not get a welcome
            // from a bar that is going away.
            _ = quit.wait_for(|quitting| *quitting) => return Ok(()),
        };
        if let Err(error) = connecting {
            // One broken connection attempt (for example a client that died while
            // connecting) must not stop the bar from accepting others: use a fresh instance.
            eprintln!("barista: a client failed to connect: {error}");
            server = next_instance().await;
            continue;
        }
        let connected = server;
        // A new instance must exist before we hand the old one away, or a client that
        // connects in between would find no pipe at all.
        server = next_instance().await;
        // Over the limit: drop the connection right away. The permit lives as long as the
        // session task, so it is given back whenever the client leaves.
        let Ok(permit) = permits.clone().try_acquire_owned() else {
            continue;
        };
        // No pid means we can't tie the client to a process: refuse rather than guess.
        let Some(pid) = pipe::client_pid(&connected) else {
            continue;
        };
        let registry = registry.clone();
        let config = SessionConfig {
            bar_version: bar_version.clone(),
            heartbeat: Heartbeat::default(),
            shutdown: quit_signal().subscribe(),
        };
        LIVE_SESSIONS.fetch_add(1, Ordering::SeqCst);
        tauri::async_runtime::spawn(async move {
            session::run(connected, Some(pid), registry, config).await;
            LIVE_SESSIONS.fetch_sub(1, Ordering::SeqCst);
            drop(permit);
        });
    }
}

/// Opens the next pipe instance, retrying until it works.
async fn next_instance() -> tokio::net::windows::named_pipe::NamedPipeServer {
    loop {
        match pipe::create_server(false) {
            Ok(next) => return next,
            Err(error) => {
                eprintln!("barista: could not open the next pipe instance: {error}");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}
