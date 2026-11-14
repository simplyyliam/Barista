# 0026: How v0.6 reads the tray (ADR 0012, built)

Status: **proposed** (2026-11-14, v0.6; becomes accepted when Liam merges the PR)

## Context

ADR 0012 chose the approach: our own hidden `Shell_TrayWnd` that sees every `Shell_NotifyIcon` message
first, forwards it to Explorer, and drives icons later by posting their callback message. It needs no DLL
injection, no memory reads and no admin rights, so it does not need the "stop and ask" step in the v0.6
goal. This ADR records how it is built and what measuring it showed.

## Decision

```text
app ─Shell_NotifyIcon─► shell32 ─FindWindow("Shell_TrayWnd")─► OUR hidden window (topmost)
                                                                │ 1. parse the message, CopyIcon
                                                                │ 2. forward the same WM_COPYDATA to Explorer
                                                                │    and return Explorer's answer
                                                                │ 3. HostEvent on a bounded channel
                                                                ▼
                            tray worker: apply to the tray state → icon to PNG (cache) → registry
                                                                ▼
                            registry (tray items, source: tray) ─► items-changed ─► the bar
```

- **Rust layout.** Win32 in `platform/windows/tray_host.rs` (window, message loop, hooks, forwarding);
  everything else is plain Rust in `tray/`: `parse.rs` (the payload), `state.rs` (the icon table and its
  limits), `icons.rs` (the PNG cache), `mod.rs` (the worker). The pure parts have unit tests.
- **Forward everything.** Every `WM_COPYDATA` goes on to Explorer, not only tray messages: the AppBar API
  (`SHAppBarMessage`, which our own bar uses to reserve space) and icon-rect queries travel the same way.
  Registered and `WM_USER` messages and `WM_COMMAND` are forwarded too. A hung Explorer costs an app at
  most one second (`SMTO_ABORTIFHUNG`, 1 s).
- **Private icon copy.** An app may free its icon right after `Shell_NotifyIcon` returns, so the host
  `CopyIcon`s it while the sender is still blocked, and the worker converts the copy later.
- **Keys.** An icon is `tray/<guid>` when it registered with a GUID, else `tray/<hwnd hex>-<id hex>`.
- **Registry.** `StatusItemView` gets `source: "app" | "tray"`. Tray items live beside client items and
  reach the UI through the same `items-changed`. A click on a tray key is a safe no-op until v0.7.
- **Cleanup.** A window that owns icons is watched with an `EVENT_OBJECT_DESTROY` hook (an event, not a
  poll): when it goes (quit or crash) its icons are removed.
- **Limits (rule 3).** 64 icons per process, 256 in total, payloads cut at 4 KB, tips at 128 characters,
  the worker queue holds 1024 events and drops the rest (counted). Dropping never stops forwarding.
- **Back off, don't fight.** If a `Shell_TrayWnd` that belongs to another program than Explorer already
  exists (StartAllBack, Seelen UI, ...), the host does not start.
- **Switch (rule 11).** `trayMode: barista | windows` in settings, default `barista`. `windows` starts no
  host: Explorer keeps the tray and the bar shows no tray items. No UI until v0.8.

## Open: keeping the top place

`FindWindow` returns the first match in z-order, and Explorer raises its taskbar now and then. The host
re-raises its own window on `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_REORDER` (no timer). Results of
the measurement are in `docs/perf/0.6.md`.

## Consequences

- Clean to remove: dropping the host closes its window and messages reach Explorer alone again.
- Apps that read child windows of the taskbar (`TrayNotifyWnd`) find none on ours. The host mirrors
  Explorer's rectangle, which is what most apps want.
- Apps that never listen for `TaskbarCreated` will not show up until they re-add their icon.
