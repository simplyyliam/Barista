# 0005: How the pipe server handles clients

Status: accepted (2026-11-09, v0.2 First item). Builds on 0001 (named pipe).

## Context

0001 chose the pipe. v0.2 builds the server, and CLAUDE.md rule 3 says one bad or slow client
must never freeze or crash the bar. Choices to make: the async model, what happens when a client
stops reading, how invalid messages are punished, and who may open the pipe.

|                                 | Task per client + bounded queue        | One thread per client | One loop for all clients        |
| ------------------------------- | -------------------------------------- | --------------------- | ------------------------------- |
| Slow client blocks others       | no (own task, queue full = disconnect) | no                    | yes unless every write is async |
| Cost per idle client            | a few KB                               | ~1 MB stack           | tiny                            |
| Fits Tauri (already runs tokio) | yes                                    | works, wasteful       | works, hard to reason about     |
| Code to read                    | straight line per client               | straight line         | state machine                   |

## Decision

**One tokio task per client** on Tauri's own runtime (`tauri::async_runtime`), with:

- **Reader loop + writer task per session.** The reader validates and updates the registry. The
  writer drains a **bounded queue of 64 messages**. Sending uses `try_send`, which never waits:
  if the queue is full the client is not reading, so the session ends. The bar never blocks on a client.
- **Frames:** 4-byte little-endian length, then JSON. A length of 0 or above 1 MB is refused before
  any allocation, and the body is read as bytes arrive, so a client that claims 1 MB and sends nothing costs nothing.
- **Handshake:** the first message must be a valid `hello` within 5 s; the protocol major version
  must match; `hello.pid` must equal the pid Windows reports for the pipe (`GetNamedPipeClientProcessId`).
- **Strikes:** every rejected message counts; 10 within 60 s sends `too_many_errors` and disconnects.
  Unknown `type` is `unknown_type`, bad JSON or fields is `invalid_message`.
- **Limits:** 64 connections at once (a semaphore), 8 items per client, text and icon sizes from the protocol crate.
- **Pipe access:** created with `PIPE_REJECT_REMOTE_CLIENTS`, a DACL that allows only the current
  user, and `first_pipe_instance(true)` so no other process can create the name before us. A second
  bar fails to start listening (logged) instead of sharing the pipe.
- A client that disconnects or is killed is removed with its items right away (EOF on the pipe).
  Heartbeat for clients that hang without closing is v0.5.

## Consequences

- A stuck client loses its connection after 64 queued messages, which is generous: the bar only
  queues replies and clicks.
- The reader has no idle timeout after `hello`; a connected but silent client costs one task until
  v0.5 adds pings. The 64-connection cap bounds that.
- Sessions are testable without Windows pipes: the session code is generic over any async stream,
  and tests use in-memory pipes (`tokio::io::duplex`).
- Windows pipe code (SDDL, pid lookup) sits in `platform/windows/pipe.rs` (CLAUDE.md rule 7).
