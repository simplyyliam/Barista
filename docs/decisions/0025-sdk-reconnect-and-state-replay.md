# 0025: The SDK reconnects by itself and replays every item

Status: accepted (2026-11-14, v0.5 Never stuck)

## Context

`connect()` rejected when the bar was not running, and after a bar restart every app lost its items for
good (our own examples called `process.exit(0)` on `close`). Rule 4: the SDK reconnects and re-registers
by itself after a bar restart, and waits if the bar is not running yet.

## Decision

**`StatusBar` is a long-lived object, not one socket.** It moves between states:

```text
            start / retry                      welcome
 waiting ───────────────► connecting ─────────────────► connected
    ▲   backoff 100 ms → 0.5 s │ fail                        │ pipe closed
    └─────────────────────────┘◄─────────────────────────────┘ → "unavailable"
 closed (for good): bar.close(), a refused hello, shutdown { too_many_errors } → "close"
```

- **Waiting is not an error.** `connect()` and `StatusItem.create()` resolve even with no bar. It
  rejects only for a refused `hello` (`version_mismatch`, a bad app name): retrying would be refused again.
  No logs while waiting.
- **Backoff** 100, 200, 400 ms, then every half second, each ±20 % (never above 0.5 s). The cap is what
  makes "items back within 2 s of the bar starting" hold with room: worst case half a second of waiting
  plus one connection attempt (it was one second at first; a bar restart measured from launch had its worst
  case right at 2.0 s). A failed `connect` on a missing pipe costs microseconds, so two a second per app
  while the bar is away is no load. The timer is not
  `unref`'d: an app that only shows an item must keep running while the bar is away.
- **Replay.** Every `StatusItem` keeps its last state (icon, label, tooltip, visible, `Menu` object with
  its mirrored `checked` flags, popover options). On each `welcome` the bar object calls `item.replay()`
  in creation order: `item.create`, `item.update { visible: false }` if hidden, `item.setMenu`,
  `item.setPopover`. The `StatusItem` objects never change, so every callback survives. Setters while
  away update the state and send nothing (no queue): the latest value is what the bar gets.
- **What dies with the connection.** An open popover window is hidden and `close` listeners run, same
  for an open menu: the bar's side of it is gone, so the app's must be too.
- **Stay-down rule.** After `shutdown { too_many_errors }` the SDK does not reconnect: replaying the
  same state would be refused the same way. `quit` and `unresponsive` reconnect.
- **`close` vs `unavailable`.** `unavailable` is "the bar went away" (and comes back: `available` fires
  again). `close` is "over for good" (`bar.close()`, refused). Existing `bar.on("close", exit)` code keeps
  working and now survives bar restarts, which is what we want.
- **`bar.close()`** sends `bye` (the bar removes the items at once, no error), ends the socket and
  destroys it after 1 s if the bar never hangs up.
- **`StatusBar.isAvailable()`**: true if the shared connection is up, else a probe (open the pipe,
  close it). The bar sees a client that left before `hello` and ignores it. It never throws.
- **`sessionId` / `barVersion`** are `undefined` while not connected.
- **Pings** are answered inside the SDK (`pong`), nothing for the app to do.

## Alternatives

- **Queue changes while away and replay them in order.** Unbounded memory for an app that updates a
  label every second while the bar is away, and the bar only needs the final state.
- **`connect()` rejects, the app retries.** Every app would write the same loop (and get it wrong).
- **Reconnect forever even after `too_many_errors`.** A noisy client would be cut off every few seconds.

## Consequences

- A client frozen over 15 s is dropped by the bar (ADR 0024); when it wakes, its pipe is closed and the
  same replay puts the items back.
- Item ids a replayed `create` conflicts with (a leftover from a session the bar has not yet cleaned
  up) cannot happen: the bar's sessions are per connection and removed when the pipe closes.
- Errors from a replayed create (for example `limit_reached`) arrive like any create error; the item
  is forgotten.
