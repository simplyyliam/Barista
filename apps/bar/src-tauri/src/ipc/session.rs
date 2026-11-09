//! One task per connected client. It reads frames, validates every message, updates the
//! registry and answers through a bounded queue. Whatever the client does, it only ever
//! affects its own task (CLAUDE.md rule 3).
//!
//! ```text
//!  client ──frames──► read loop ─► validate ─► registry
//!  client ◄─frames─── writer task ◄─ bounded queue (64) ◄─ replies, item.click
//! ```

use super::frame::{read_frame, write_frame, FrameError};
use super::validate;
use crate::registry::{ClickError, ItemPatch, Outbox, Registry, RegistryError, SessionId};
use barista_protocol::{
    is_compatible, BarMessage, ClientMessage, ErrorCode, MAX_ITEMS_PER_CLIENT, MAX_LABEL_LEN,
    MAX_TOOLTIP_LEN, PROTOCOL_VERSION,
};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio::time::timeout;

/// A client has this long to send `hello` after connecting.
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
/// Replies waiting to be written. If a client lets this fill up, it is cut off.
const OUTBOX_SIZE: usize = 64;
/// 10 rejected messages within a minute and the client is disconnected.
const STRIKE_LIMIT: usize = 10;
const STRIKE_WINDOW: Duration = Duration::from_secs(60);

/// `debug.*` messages exist in debug builds, and in release builds only when the bar was
/// started with `BARISTA_DEBUG` set (the bench does that). Otherwise any program of the same
/// user could list every app's items.
fn debug_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| cfg!(debug_assertions) || std::env::var_os("BARISTA_DEBUG").is_some())
}

/// Counts rejected messages in a sliding window.
#[derive(Default)]
struct Strikes(VecDeque<Instant>);

impl Strikes {
    /// Records one strike. True means the limit is reached: disconnect.
    fn record(&mut self, now: Instant) -> bool {
        while self
            .0
            .front()
            .is_some_and(|t| now.duration_since(*t) > STRIKE_WINDOW)
        {
            self.0.pop_front();
        }
        self.0.push_back(now);
        self.0.len() >= STRIKE_LIMIT
    }
}

/// A message the bar refuses. It becomes `error { code, message }`.
struct Reject {
    request_id: Option<String>,
    code: ErrorCode,
    message: String,
}

/// What to do after handling one message.
enum Step {
    Ok,
    Reject(Reject),
    /// The client cannot be written to any more (queue full or connection gone).
    Disconnect,
}

/// Queues a message for the writer. False = the queue is full or the writer is gone, so the
/// session must end. `try_send` never waits, which is what keeps a slow client harmless.
fn send(tx: &Outbox, message: BarMessage) -> bool {
    tx.try_send(message).is_ok()
}

fn send_error(
    tx: &Outbox,
    request_id: Option<String>,
    code: ErrorCode,
    message: impl Into<String>,
) -> bool {
    send(
        tx,
        BarMessage::Error {
            request_id,
            code,
            message: message.into(),
        },
    )
}

/// Runs a whole connection. `client_pid` comes from the pipe itself (`None` in tests that
/// don't have a real pipe) and must match the pid in `hello`.
pub async fn run<S>(
    stream: S,
    client_pid: Option<u32>,
    registry: Arc<Registry>,
    bar_version: String,
) where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    let (tx, mut rx) = mpsc::channel::<BarMessage>(OUTBOX_SIZE);

    let mut writer_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let Ok(json) = serde_json::to_vec(&message) else {
                continue;
            };
            if write_frame(&mut writer, &json).await.is_err() {
                break; // client is gone; dropping `rx` makes later sends fail
            }
        }
    });

    serve(&mut reader, &tx, client_pid, &registry, &bar_version).await;

    // `serve` removed the session, so the registry no longer holds a sender. Dropping ours
    // lets the writer finish what is queued (like a last error message) and stop.
    drop(tx);
    if timeout(Duration::from_secs(2), &mut writer_task)
        .await
        .is_err()
    {
        writer_task.abort(); // the client stopped reading: do not wait for it
    }
}

async fn serve<R: AsyncRead + Unpin>(
    reader: &mut R,
    tx: &Outbox,
    client_pid: Option<u32>,
    registry: &Arc<Registry>,
    bar_version: &str,
) {
    let Some(session) = handshake(reader, tx, client_pid, registry, bar_version).await else {
        return;
    };
    let mut strikes = Strikes::default();
    loop {
        // If the writer died (client stopped reading or left), end the session too, so its
        // items don't stay in the bar. Dropping the read here is fine: we are leaving anyway.
        let frame = tokio::select! {
            frame = read_frame(reader) => frame,
            () = tx.closed() => break,
        };
        let bytes = match frame {
            Ok(bytes) => bytes,
            Err(FrameError::BadLength(n)) => {
                send_error(
                    tx,
                    None,
                    ErrorCode::InvalidMessage,
                    format!("frame length {n} is not allowed"),
                );
                break;
            }
            Err(_) => break, // hung up (or the connection broke)
        };
        let step = match validate::parse_message(&bytes) {
            Ok(message) => handle(session, message, registry, tx),
            Err(e) => Step::Reject(Reject {
                request_id: e.request_id,
                code: e.code,
                message: e.message,
            }),
        };
        match step {
            Step::Ok => {}
            Step::Disconnect => break,
            Step::Reject(r) => {
                if !send_error(tx, r.request_id, r.code, r.message) {
                    break;
                }
                if strikes.record(Instant::now()) {
                    send_error(
                        tx,
                        None,
                        ErrorCode::TooManyErrors,
                        "too many invalid messages, disconnecting",
                    );
                    break;
                }
            }
        }
    }
    registry.remove_session(session);
}

/// First message must be a valid `hello`. Returns the new session, or None after telling the
/// client why it was refused.
async fn handshake<R: AsyncRead + Unpin>(
    reader: &mut R,
    tx: &Outbox,
    client_pid: Option<u32>,
    registry: &Arc<Registry>,
    bar_version: &str,
) -> Option<SessionId> {
    let refuse = |request_id, code, message: &str| {
        send_error(tx, request_id, code, message);
        None
    };
    let bytes = match timeout(HELLO_TIMEOUT, read_frame(reader)).await {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(FrameError::BadLength(n))) => {
            return refuse(
                None,
                ErrorCode::InvalidMessage,
                &format!("frame length {n} is not allowed"),
            )
        }
        _ => return None, // hung up, broken, or too slow
    };
    let message = match validate::parse_message(&bytes) {
        Ok(message) => message,
        Err(e) => return refuse(e.request_id, e.code, &e.message),
    };
    let request_id = message.request_id().map(str::to_string);
    let ClientMessage::Hello {
        protocol_version,
        app_name,
        pid,
        ..
    } = message
    else {
        return refuse(request_id, ErrorCode::NotReady, "send hello first");
    };
    if !is_compatible(&protocol_version) {
        return refuse(
            request_id,
            ErrorCode::VersionMismatch,
            &format!(
                "the bar speaks protocol {PROTOCOL_VERSION}, the client sent {protocol_version:?}"
            ),
        );
    }
    if let Err(message) = validate::app_name(&app_name) {
        return refuse(request_id, ErrorCode::InvalidMessage, &message);
    }
    // A client may not claim to be another process: the pid must be the one on the pipe.
    if client_pid.is_some_and(|real| real != pid) {
        return refuse(
            request_id,
            ErrorCode::InvalidMessage,
            "pid does not match the connection",
        );
    }
    let session = registry.add_session(app_name, pid, tx.clone());
    let welcome = BarMessage::Welcome {
        session_id: format!("s{session}"),
        bar_version: bar_version.to_string(),
        protocol_version: PROTOCOL_VERSION.to_string(),
    };
    if !send(tx, welcome) {
        registry.remove_session(session);
        return None;
    }
    Some(session)
}

fn reject(request_id: &Option<String>, code: ErrorCode, message: impl Into<String>) -> Step {
    Step::Reject(Reject {
        request_id: request_id.clone(),
        code,
        message: message.into(),
    })
}

fn registry_reject(request_id: &Option<String>, error: RegistryError, id: &str) -> Step {
    let message = match error {
        RegistryError::DuplicateId => format!("item {id:?} already exists"),
        RegistryError::NotFound => format!("no item {id:?}"),
        RegistryError::LimitReached => format!("at most {MAX_ITEMS_PER_CLIENT} items per client"),
    };
    reject(request_id, error.code(), message)
}

/// Checks the optional text/icon fields shared by `item.create` and `item.update`.
fn check_fields(
    request_id: &Option<String>,
    icon: &Option<String>,
    label: &Option<String>,
    tooltip: &Option<String>,
) -> Result<(), Step> {
    if let Some(icon) = icon {
        validate::icon(icon).map_err(|m| reject(request_id, ErrorCode::InvalidIcon, m))?;
    }
    if let Some(label) = label {
        validate::text("label", label, MAX_LABEL_LEN)
            .map_err(|m| reject(request_id, ErrorCode::InvalidMessage, m))?;
    }
    if let Some(tooltip) = tooltip {
        validate::text("tooltip", tooltip, MAX_TOOLTIP_LEN)
            .map_err(|m| reject(request_id, ErrorCode::InvalidMessage, m))?;
    }
    Ok(())
}

fn handle(session: SessionId, message: ClientMessage, registry: &Registry, tx: &Outbox) -> Step {
    let rid = message.request_id().map(str::to_string);
    let result = match message {
        ClientMessage::Hello { .. } => Err(reject(
            &rid,
            ErrorCode::InvalidMessage,
            "hello was already sent",
        )),
        ClientMessage::ItemCreate {
            id,
            icon,
            label,
            tooltip,
            ..
        } => (|| {
            validate::item_id(&id).map_err(|m| reject(&rid, ErrorCode::InvalidMessage, m))?;
            check_fields(&rid, &icon, &label, &tooltip)?;
            registry
                .create(session, id.clone(), icon, label, tooltip)
                .map_err(|e| registry_reject(&rid, e, &id))
        })(),
        ClientMessage::ItemUpdate {
            id,
            icon,
            label,
            tooltip,
            visible,
            ..
        } => (|| {
            validate::item_id(&id).map_err(|m| reject(&rid, ErrorCode::InvalidMessage, m))?;
            check_fields(&rid, &icon, &label, &tooltip)?;
            registry
                .update(
                    session,
                    &id,
                    ItemPatch {
                        icon,
                        label,
                        tooltip,
                        visible,
                    },
                )
                .map_err(|e| registry_reject(&rid, e, &id))
        })(),
        ClientMessage::ItemRemove { id, .. } => (|| {
            validate::item_id(&id).map_err(|m| reject(&rid, ErrorCode::InvalidMessage, m))?;
            registry
                .remove(session, &id)
                .map_err(|e| registry_reject(&rid, e, &id))
        })(),
        ClientMessage::DebugListItems { .. } | ClientMessage::DebugClick { .. }
            if !debug_enabled() =>
        {
            // Pretend these messages do not exist.
            Err(reject(&rid, ErrorCode::UnknownType, "unknown message type"))
        }
        ClientMessage::DebugListItems { .. } => {
            let items = registry.debug_items();
            if send(
                tx,
                BarMessage::DebugItems {
                    request_id: rid.clone(),
                    items,
                },
            ) {
                Ok(())
            } else {
                Err(Step::Disconnect)
            }
        }
        ClientMessage::DebugClick { id, button, .. } => {
            // `id` is the registry key (`s3/counter`), so a test tool can click any item.
            match registry.click(&id, button) {
                Ok(()) => Ok(()),
                Err(ClickError::NotFound) => {
                    Err(reject(&rid, ErrorCode::NotFound, format!("no item {id:?}")))
                }
                // The target is stuck, not the sender: answer with an error but give no strike
                // (`Err(Step::Ok)` just means "stop here, nothing more to do").
                Err(ClickError::Unreachable) => {
                    let message = format!("item {id:?} cannot be reached right now");
                    if send_error(tx, rid.clone(), ErrorCode::NotFound, message) {
                        Err(Step::Ok)
                    } else {
                        Err(Step::Disconnect)
                    }
                }
            }
        }
    };
    result.map_or_else(|step| step, |()| Step::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use barista_protocol::{ItemInfo, MouseButton};
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde_json::json;
    use tokio::io::{AsyncWriteExt, DuplexStream};

    struct Client {
        stream: DuplexStream,
        registry: Arc<Registry>,
    }

    /// Starts a session on an in-memory pipe. `duplex_size` is small in the slow-client test.
    fn connect_with(registry: &Arc<Registry>, pid: Option<u32>, duplex_size: usize) -> Client {
        let (ours, theirs) = tokio::io::duplex(duplex_size);
        tokio::spawn(run(theirs, pid, registry.clone(), "9.9.9".into()));
        Client {
            stream: ours,
            registry: registry.clone(),
        }
    }

    fn connect(registry: &Arc<Registry>) -> Client {
        connect_with(registry, Some(4242), 1 << 20)
    }

    impl Client {
        async fn send_raw(&mut self, bytes: &[u8]) {
            write_frame(&mut self.stream, bytes).await.unwrap();
        }

        async fn send(&mut self, value: serde_json::Value) {
            self.send_raw(value.to_string().as_bytes()).await;
        }

        async fn hello(&mut self) -> BarMessage {
            self.send(hello_json(4242, PROTOCOL_VERSION)).await;
            self.recv().await
        }

        async fn recv(&mut self) -> BarMessage {
            let bytes = timeout(Duration::from_secs(2), read_frame(&mut self.stream))
                .await
                .expect("timed out waiting for the bar")
                .expect("connection closed");
            serde_json::from_slice(&bytes).unwrap()
        }

        /// Replies come in order, so asking for the item list proves everything sent before it
        /// was processed.
        async fn items(&mut self) -> Vec<ItemInfo> {
            self.send(json!({"type": "debug.listItems"})).await;
            match self.recv().await {
                BarMessage::DebugItems { items, .. } => items,
                other => panic!("expected debug.items, got {other:?}"),
            }
        }

        async fn expect_error(&mut self) -> (ErrorCode, Option<String>) {
            match self.recv().await {
                BarMessage::Error {
                    code, request_id, ..
                } => (code, request_id),
                other => panic!("expected an error, got {other:?}"),
            }
        }

        /// The bar closed the connection.
        async fn expect_closed(&mut self) {
            let result = timeout(Duration::from_secs(2), read_frame(&mut self.stream))
                .await
                .expect("not closed");
            assert!(matches!(result, Err(FrameError::Closed)), "{result:?}");
        }
    }

    fn hello_json(pid: u32, version: &str) -> serde_json::Value {
        json!({"type":"hello","protocolVersion":version,"appName":"Test","pid":pid,
               "sdk":{"name":"t","version":"0"}})
    }

    fn png_url(extra: usize) -> String {
        let mut data = b"\x89PNG\r\n\x1a\n".to_vec();
        data.extend_from_slice(&13u32.to_be_bytes());
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend(std::iter::repeat(0u8).take(extra));
        format!("data:image/png;base64,{}", STANDARD.encode(data))
    }

    async fn eventually(mut check: impl FnMut() -> bool) {
        for _ in 0..100 {
            if check() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("condition never became true");
    }

    #[tokio::test]
    async fn hello_create_update_remove() {
        let registry = Arc::new(Registry::new());
        let mut c = connect(&registry);
        assert_eq!(
            c.hello().await,
            BarMessage::Welcome {
                session_id: "s1".into(),
                bar_version: "9.9.9".into(),
                protocol_version: PROTOCOL_VERSION.into()
            }
        );
        c.send(json!({"type":"item.create","id":"a","label":"0","icon":png_url(4)}))
            .await;
        let items = c.items().await;
        assert_eq!(
            (items[0].id.as_str(), items[0].label.as_deref()),
            ("s1/a", Some("0"))
        );
        assert!(items[0].has_icon);
        c.send(json!({"type":"item.update","id":"a","label":"1"}))
            .await;
        assert_eq!(c.items().await[0].label.as_deref(), Some("1"));
        c.send(json!({"type":"item.remove","id":"a"})).await;
        assert!(c.items().await.is_empty());
    }

    #[tokio::test]
    async fn bad_messages_are_rejected_and_the_session_keeps_working() {
        let registry = Arc::new(Registry::new());
        let mut c = connect(&registry);
        c.hello().await;

        c.send(json!({"type":"item.create","id":"a","icon":png_url(300 * 1024),"requestId":"r1"}))
            .await;
        assert_eq!(
            c.expect_error().await,
            (ErrorCode::InvalidIcon, Some("r1".into()))
        );
        c.send(json!({"type":"item.create","id":"Bad Id"})).await;
        assert_eq!(c.expect_error().await.0, ErrorCode::InvalidMessage);
        c.send(json!({"type":"item.create","id":"a","label":"x".repeat(500)}))
            .await;
        assert_eq!(c.expect_error().await.0, ErrorCode::InvalidMessage);
        c.send_raw(b"{not json").await;
        assert_eq!(c.expect_error().await.0, ErrorCode::InvalidMessage);
        c.send(json!({"type":"item.teleport","requestId":"r2"}))
            .await;
        assert_eq!(
            c.expect_error().await,
            (ErrorCode::UnknownType, Some("r2".into()))
        );
        c.send(json!({"type":"item.update","id":"ghost"})).await;
        assert_eq!(c.expect_error().await.0, ErrorCode::NotFound);
        c.send(json!(hello_json(4242, PROTOCOL_VERSION))).await;
        assert_eq!(c.expect_error().await.0, ErrorCode::InvalidMessage);

        // Nothing leaked into the registry, and a good message still works.
        assert!(c.items().await.is_empty());
        c.send(json!({"type":"item.create","id":"a"})).await;
        c.send(json!({"type":"item.create","id":"a","requestId":"r3"}))
            .await;
        assert_eq!(
            c.expect_error().await,
            (ErrorCode::DuplicateId, Some("r3".into()))
        );
        assert_eq!(c.items().await.len(), 1);
    }

    #[tokio::test]
    async fn item_limit_per_client() {
        let registry = Arc::new(Registry::new());
        let mut c = connect(&registry);
        c.hello().await;
        for n in 0..MAX_ITEMS_PER_CLIENT {
            c.send(json!({"type":"item.create","id":format!("i{n}")}))
                .await;
        }
        c.send(json!({"type":"item.create","id":"one-too-many"}))
            .await;
        assert_eq!(c.expect_error().await.0, ErrorCode::LimitReached);
    }

    #[tokio::test]
    async fn oversized_frame_is_refused_and_closed() {
        let registry = Arc::new(Registry::new());
        let mut c = connect(&registry);
        c.hello().await;
        c.stream
            .write_all(&(2u32 * 1024 * 1024).to_le_bytes())
            .await
            .unwrap();
        assert_eq!(c.expect_error().await.0, ErrorCode::InvalidMessage);
        c.expect_closed().await;
    }

    #[tokio::test]
    async fn handshake_refusals() {
        let registry = Arc::new(Registry::new());

        let mut c = connect(&registry);
        c.send(hello_json(4242, "1.0")).await;
        assert_eq!(c.expect_error().await.0, ErrorCode::VersionMismatch);
        c.expect_closed().await;

        let mut c = connect(&registry);
        c.send(hello_json(1, PROTOCOL_VERSION)).await; // pipe says the client is 4242
        assert_eq!(c.expect_error().await.0, ErrorCode::InvalidMessage);
        c.expect_closed().await;

        let mut c = connect(&registry);
        c.send(json!({"type":"item.create","id":"a"})).await;
        assert_eq!(c.expect_error().await.0, ErrorCode::NotReady);
        c.expect_closed().await;

        let mut c = connect(&registry);
        c.send_raw(b"garbage").await;
        assert_eq!(c.expect_error().await.0, ErrorCode::InvalidMessage);
        c.expect_closed().await;

        assert_eq!(registry.session_count(), 0);
    }

    #[tokio::test]
    async fn a_noisy_client_is_cut_off_and_others_are_unaffected() {
        let registry = Arc::new(Registry::new());
        let mut good = connect(&registry);
        good.hello().await;
        good.send(json!({"type":"item.create","id":"keep"})).await;
        let mut bad = connect(&registry);
        bad.hello().await;
        for _ in 0..STRIKE_LIMIT {
            bad.send_raw(b"nope").await;
        }
        for _ in 0..STRIKE_LIMIT {
            assert_eq!(bad.expect_error().await.0, ErrorCode::InvalidMessage);
        }
        assert_eq!(bad.expect_error().await.0, ErrorCode::TooManyErrors);
        bad.expect_closed().await;

        assert_eq!(good.items().await.len(), 1);
        good.send(json!({"type":"item.create","id":"more"})).await;
        assert_eq!(good.items().await.len(), 2);
    }

    #[test]
    fn strikes_expire_after_the_window() {
        let start = Instant::now();
        let mut strikes = Strikes::default();
        for n in 0..STRIKE_LIMIT - 1 {
            assert!(!strikes.record(start + Duration::from_secs(n as u64)));
        }
        // The next one comes long after the others, which have expired.
        assert!(!strikes.record(start + Duration::from_secs(200)));
        let mut strikes = Strikes::default();
        let hits = (0..STRIKE_LIMIT)
            .filter(|n| strikes.record(start + Duration::from_secs(*n as u64)))
            .count();
        assert_eq!(hits, 1);
    }

    #[tokio::test]
    async fn disconnect_removes_the_items() {
        let registry = Arc::new(Registry::new());
        let mut c = connect(&registry);
        c.hello().await;
        c.send(json!({"type":"item.create","id":"a"})).await;
        assert_eq!(c.items().await.len(), 1);
        drop(c.stream);
        eventually(|| registry.snapshot().is_empty() && registry.session_count() == 0).await;
    }

    #[tokio::test]
    async fn clicks_reach_the_client() {
        let registry = Arc::new(Registry::new());
        let mut c = connect(&registry);
        c.hello().await;
        c.send(json!({"type":"item.create","id":"a"})).await;
        c.items().await;
        c.registry.click("s1/a", MouseButton::Left).unwrap();
        assert_eq!(
            c.recv().await,
            BarMessage::ItemClick {
                id: "a".into(),
                button: MouseButton::Left
            }
        );
        c.send(json!({"type":"debug.click","id":"s1/a","button":"right"}))
            .await;
        assert_eq!(
            c.recv().await,
            BarMessage::ItemClick {
                id: "a".into(),
                button: MouseButton::Right
            }
        );
        c.send(json!({"type":"debug.click","id":"s1/ghost","button":"right"}))
            .await;
        assert_eq!(c.expect_error().await.0, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn a_client_that_never_reads_is_dropped_without_blocking_anyone() {
        let registry = Arc::new(Registry::new());
        let mut other = connect(&registry);
        other.hello().await;

        let mut slow = connect_with(&registry, Some(4242), 128);
        slow.send(hello_json(4242, PROTOCOL_VERSION)).await;
        // Never read replies, keep asking for more: the queue fills and the session ends.
        for _ in 0..400 {
            let ask = json!({"type": "debug.listItems"}).to_string();
            // Once the bar cuts us off the write fails: that is the expected outcome.
            if write_frame(&mut slow.stream, ask.as_bytes()).await.is_err() {
                break;
            }
        }
        eventually(|| registry.session_count() == 1).await;

        // The well-behaved client was not slowed down.
        other.send(json!({"type":"item.create","id":"a"})).await;
        assert_eq!(other.items().await.len(), 1);
    }

    #[tokio::test]
    async fn clicking_a_stuck_client_does_not_disconnect_the_sender() {
        let registry = Arc::new(Registry::new());
        let mut stuck = connect_with(&registry, Some(4242), 128);
        stuck.hello().await;
        stuck.send(json!({"type":"item.create","id":"a"})).await;
        stuck.items().await;
        // From here `stuck` never reads: its queue fills up.
        let mut tool = connect(&registry);
        tool.hello().await;
        let mut unreachable = 0;
        for _ in 0..200 {
            tool.send(json!({"type":"debug.click","id":"s1/a","button":"left"})).await;
            // A reply only exists once the target's queue is full.
            if tokio::time::timeout(Duration::from_millis(5), tool.recv()).await.is_ok() {
                unreachable += 1;
            }
        }
        assert!(unreachable > 10, "expected many unreachable errors, got {unreachable}");
        // More than STRIKE_LIMIT errors came back and the sender is still connected.
        assert_eq!(tool.items().await.len(), 1);
    }
}
