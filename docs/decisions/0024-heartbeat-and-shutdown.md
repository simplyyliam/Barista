# 0024: The bar pings its clients and says goodbye when it quits

Status: accepted (2026-11-14, v0.5 Never stuck)

## Context

Until v0.5 the bar only noticed a client that closed its pipe. A client that hangs (stuck loop,
suspended process, debugger) keeps its icons forever; the only limit was the 64-connection cap. And
when the bar quits, clients found out only by a broken pipe, with no reason.

## Decision

**Heartbeat.** The bar sends `ping { seq }` every 5 s. The client answers `pong { seq }`. If no
`pong` has come for 15 s the session ends: best-effort `shutdown { unresponsive }`, then
`registry.remove_session` removes the items and the popover and menu close through the existing sync.
A hung client therefore loses its icons 15 to 20 s after its last answer (the check runs on the 5 s tick).

- A `pong` only counts for a `seq` the bar really sent. Any other `seq` is `invalid_message` (a
  strike), so a client cannot keep itself alive by sending pongs blindly.
- **Only clients that say protocol 0.5 or newer** in `hello` get pings (`minor_version`). A 0.4 client
  never answers, and rule 2 says it must keep working within major 0. Those clients stay bounded by the
  connection cap as before.
- Pipe disconnect already removed items at once (v0.2); `bye { }` is the polite version of the same:
  no error, no strike, the session ends immediately.

**One read task per session.** The session loop now wakes for four reasons: a frame, the heartbeat
tick, the bar quitting, and the writer dying. `read_frame` is not cancel-safe (dropping it half-way loses
the bytes already read), so putting it inside a `select!` next to a timer would corrupt a frame whenever
a tick fired mid-read. A small task owns the read half and sends whole frames through a channel of one
(backpressure: a flooding client waits in the pipe, not in memory). Receiving from a channel is
cancel-safe. The task is aborted when the session ends, because a hung client may never finish a read.
A test sends half a frame across several ticks.

**Shutdown.** `shutdown { reason }` is the bar's last message before it hangs up:

| `reason` | When | SDK |
|---|---|---|
| `quit` | the bar exits (`RunEvent::Exit`) | waits and reconnects |
| `unresponsive` | no `pong` in 15 s | reconnects (best effort: a hung client never reads it) |
| `too_many_errors` | 10 invalid messages in a minute | stays down (same messages would be refused again) |

`error { too_many_errors }` is still sent first, so 0.4 SDKs (which ignore unknown message types) behave
as before.

On exit `ipc::shutdown(300 ms)` flips a `watch` signal: every session sends `shutdown { quit }` and ends,
the accept loop stops (the pipe closes, so a retrying SDK does not get a welcome from a dying bar), and
the main thread waits at most 300 ms for the writers to flush (a live-session counter). Quitting is
`app.exit(0)`; the new command `quit_bar` exists for a future "Quit Barista" entry (no UI yet). A bar that
is killed (taskkill, crash) sends nothing: clients see the pipe break, which they handle the same way.

**Timings are a config, not constants in the loop.** `SessionConfig { bar_version, heartbeat, shutdown }`
lets the tests run the heartbeat in milliseconds.

## Alternatives

- **`tokio::time::timeout` around `read_frame`.** Same cancel-safety problem; also only detects silence,
  not a client that reads nothing.
- **TCP-style keepalive on the pipe.** Named pipes have none; a hung process keeps its handle open.
- **Heartbeat for 0.4 clients too.** Would drop every old SDK after 15 s. Maybe at protocol 1.0.

## Consequences

- The idle cost is one timer per client; no polling (rule 9).
- Hand-written clients that claim 0.5 must answer pings (`examples/inspect` does). The bench and
  `scripts/panel-latency.ps1` claim 0.4 and are exempt.
- A client suspended for more than 15 s (laptop sleep) loses its items; the SDK puts them back when it
  wakes and notices the closed pipe (ADR 0025).
