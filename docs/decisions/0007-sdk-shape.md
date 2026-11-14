# 0007: Shape of the SDK's connection and items

Status: accepted (2026-11-09, v0.2 First item)

## Context

`@baristabar/sdk` is the public face of the protocol. v0.2 needs connect, `hello` and `StatusItem`.
Questions: does the developer manage a connection, and how do refusals from the bar reach them,
given that the bar only answers with errors (successes have no reply)?

## Decision

- **A shared default connection.** `BA.connect(options?)` connects once and returns the same `StatusBar`
  for every later call; `BA.StatusItem.create` calls it for you. Simple apps never see a connection;
  apps that care (name, "bar went away") call `connect` first. The class `StatusBar` is the host in
  the naming from CLAUDE.md; passing `bar` to `create` allows more than one connection (tests).
- **`create` resolves once the message is sent**, not when the bar accepted it. The bar has no success
  reply on purpose (less traffic, simpler protocol). The SDK mirrors the bar's rules (id, text, icon
  type and size) and throws at the call, so the refusals that remain are rare: `limit_reached`, an
  SVG/PNG the bar judges invalid.
- **Refusals come back as events.** Every request carries `requestId = "<itemId>#<n>"`. An `error`
  with that id goes to `item.on("error")`; without a listener it goes to `bar.on("error")`; without
  one of those it is printed with `console.warn`. Nothing is silently lost, and nothing throws in a timer.
- **Setters are fire-and-forget** (`setLabel` returns `void`): they are called from timers and handlers, and awaiting each
  would add nothing. After the connection closes they do nothing, and `bar.on("close")` says so; reconnect is v0.5.
- **Both names are exported:** `BA.StatusItem` and `BAStatusItem` (also `BAStatusBar`, `BAConnect`).
- **Icons accept bytes or a data URL.** Bytes are sniffed (PNG signature, `<svg`/`<?xml`), so callers pass
  `readFileSync("icon.svg")` without thinking about MIME types.
- **No runtime dependencies:** `node:net` only. The framing is about 40 lines (`frame.ts`).

## Consequences

- A typo in an icon file fails at `create` (good), but "the bar had no room" shows up a moment later as an event.
- The default connection is per process. An app with two separate connections must pass `bar` itself.
- When reconnect arrives (v0.5), the shared `StatusBar` can re-register `bar.items` by itself, because it
  already keeps them.
