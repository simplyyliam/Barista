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
  poll): when it goes (quit or crash) its icons are removed. (The plan said "wait on the process handle"; the
  hook also covers an app that destroys its window but keeps running.)
- **Limits (rule 3).** 64 icons per process, 256 in total, payloads cut at 4 KB, tips at 128 characters,
  the worker queue holds 1024 events and drops the rest (counted). Dropping never stops forwarding.
- **Back off, don't fight.** If a `Shell_TrayWnd` that belongs to another program than Explorer already
  exists (StartAllBack, Seelen UI, ...), the host does not start.
- **Switch (rule 11).** `trayMode: barista | windows` in settings, default `barista`. `windows` starts no
  host: Explorer keeps the tray and the bar shows no tray items. No UI until v0.8.

## Keeping the top place (measured)

`FindWindow` returns the first match in z-order, and Explorer raises its taskbar now and then. The host
re-raises its own window on `EVENT_SYSTEM_FOREGROUND` and `EVENT_OBJECT_REORDER`, and when Explorer
restarts (`TaskbarCreated`). No timer. `scripts/tray-zorder.ps1` samples `FindWindow` every 50 ms while
it does the things that make Explorer raise the taskbar (3 rounds each), on Windows 11 build 26300:

| Trigger | Explorer first | Result |
|---|---|---|
| Win+D, Win+B, Win+T, Alt+Tab | 0 of ~58 samples each | the events keep us on top |
| Start menu open | ~55 of ~58 (the whole time it is open) | cannot be won; back on top once Start closes |

A 100 ms retry timer while we were not first made no difference during Start (the taskbar seems to sit in
a higher z-band while Start is up), so it was removed again: rule 9, and the data did not justify it.

**Known gap:** a tray message sent while Start is open reaches Explorer only. A new icon added in that
moment is not shown until its app adds it again; a removed icon whose window survives stays as a ghost
until it is re-added or its window is destroyed. Apps launched from Start add their icon after Start has
closed, so the common case is covered. If dogfooding shows ghosts, the fix is a `TaskbarCreated`
re-broadcast when we get back on top after a loss (it makes every app re-add, so only worth it if needed).

## Consequences

- Clean to remove: dropping the host closes its window and messages reach Explorer alone again.
- Apps that read child windows of the taskbar (`TrayNotifyWnd`) find none on ours. The host mirrors
  Explorer's rectangle, which is what most apps want.
- Apps that never listen for `TaskbarCreated` will not show up until they re-add their icon.
