# 0030: First-party items and the Windows helper

Status: **proposed** (2026-11-16, v0.8; becomes accepted when Liam merges the PR)

## Context

v0.8 ships mic mute, now playing and a system monitor "built only on the public SDK", to prove the SDK.
They are not bundled with the bar: people can build their own, and other SDKs are coming. Node cannot reach
the microphone's mute switch or the Windows media controls.

## Decision

- **Three separate small apps** in `examples/`: `mic-mute` and `now-playing` (Node), `system-monitor`
  (Electron, because it needs a popover window). Each uses `@baristabar/sdk` and nothing from the bar.
- **A tiny Rust helper**, `examples/win-helper` (`win-helper --mic` / `--media`), for the two Windows
  APIs. It speaks JSON lines (events on stdout, commands on stdin), is event-driven (COM callbacks and WinRT
  events, no polling) and exits when stdin closes, so it never outlives its app. Lines over 1 KB are ignored.
  Measured: 2 to 3 MB each. koffi (COM from JS) and a PowerShell child (about 60 MB, polling) were rejected.
- **The system monitor samples in Node** (`os.cpus()` deltas, `os.freemem()`), once a second, label only
  touched when its text changes. The popover is an Electron window with its look from the "Found on Cosmos"
  design: dot-matrix numerals, a dot history graph, per-core halftone dots, a lime memory gauge. Its
  memory is recorded in `docs/perf/0.8.md` and is not counted in the bar's budget, because it is a
  client app like any other. If a future measurement says the sampler itself is too costly, move the
  numbers into Rust and let the bar draw them (the release file's escape hatch); not needed now.
- **Click rule:** an item with a menu opens it on left click, so mic mute has no menu (one click toggles).
  The dogfooding gap (left action plus right menu on one item) is on the Fix next list.

## Consequences

- The helper is built with `pnpm win-helper` (`cargo build -p barista-win-helper --release`) and found next
  to the repo or through `BARISTA_WIN_HELPER`. Packaging it with a future installer is out of scope.
