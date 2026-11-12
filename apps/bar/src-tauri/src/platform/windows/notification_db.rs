//! Reads the toast XML from Windows' own notification database, read-only (ADR 0018).
//!
//! The public API hides images, the attribution line and the buttons; the database has them.
//! It is not a documented format, so every failure here just means "text only": the panel still
//! works. We never write to it. A read in WAL mode does not block Windows from writing, it only
//! keeps the file from being trimmed while the query runs (a few milliseconds).

use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// A toast bigger than this is not real: skip it.
const MAX_XML_BYTES: usize = 256 * 1024;

/// `%LOCALAPPDATA%\Microsoft\Windows\Notifications`, the folder to watch.
pub fn folder() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(
        PathBuf::from(local)
            .join("Microsoft")
            .join("Windows")
            .join("Notifications"),
    )
}

/// Toast id to XML text, for every toast currently stored. Empty if the database can't be read.
pub fn read_toasts() -> HashMap<u32, String> {
    read().unwrap_or_default()
}

fn read() -> Option<HashMap<u32, String>> {
    let path = folder()?.join("wpndatabase.db");
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    conn.busy_timeout(Duration::from_millis(200)).ok()?;
    let mut stmt = conn
        .prepare("SELECT Id, Payload FROM Notification WHERE Type = 'toast'")
        .ok()?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .ok()?;
    let mut out = HashMap::new();
    for (id, payload) in rows.flatten() {
        if let (Ok(id), true) = (u32::try_from(id), payload.len() <= MAX_XML_BYTES) {
            out.insert(id, String::from_utf8_lossy(&payload).into_owned());
        }
    }
    Some(out)
}

/// A number that changes when a toast arrives, goes away or is replaced, and not otherwise. Cheap
/// (ids and arrival times only), so the folder watcher can ask it on every write. It matters
/// because reading through the listener writes to this database itself: without this check our own
/// read would wake the watcher, which would read again, forever. `None` if the database can't be read.
pub fn fingerprint() -> Option<u64> {
    use std::hash::{Hash, Hasher};
    let path = folder()?.join("wpndatabase.db");
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
    conn.busy_timeout(Duration::from_millis(200)).ok()?;
    let mut stmt = conn
        .prepare("SELECT Id, ArrivalTime FROM Notification WHERE Type = 'toast' ORDER BY Id")
        .ok()?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
        .ok()?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for row in rows.flatten() {
        row.hash(&mut hasher);
    }
    Some(hasher.finish())
}
