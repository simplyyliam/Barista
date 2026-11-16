# 0032: Restartable tray host

Status: **proposed** (2026-11-16, v0.8; becomes accepted when Liam merges the PR)

## Context

ADR 0026 hosts the tray with a hidden `Shell_TrayWnd`. The sink was a `OnceLock` and the window class was
never unregistered, so the host could start once per process and `trayMode` only applied at the next start.
The settings window flips `barista | windows` while the bar runs (rule 11).

## Decision

- The event sink is a `Mutex<Option<SyncSender>>`: `start` fails only while one is set, and dropping the host
  clears it (and the watched-window set). The window class is unregistered when the host's message loop ends.
- `tray::set_mode(mode)`: `windows` closes the host (Explorer gets every message again) and empties the
  bar's tray icons; `barista` starts it again, which broadcasts `TaskbarCreated`, so running apps add their
  icons back. The registry is kept in a static for that.
- Tray owner or pid changes no longer wake the UI (they only matter for clicks).

## Consequences

- Verified live with 11 real tray icons: Windows mode gave 0, Barista mode brought 11 back in under 8 s.
