# 0031: Taskbar auto-hide and start with Windows

Status: **proposed** (2026-11-16, v0.8; becomes accepted when Liam merges the PR)

## Context

"Hide the system tray or auto-hide the taskbar": the tray icons already live in the bar, but Windows 11 has
no supported way to hide only the tray corner. Start with Windows needs a startup entry.

## Decision

- **Taskbar:** one switch that turns Windows' own auto-hide on (`SHAppBarMessage(ABM_SETSTATE)` on the
  taskbar, platform module `taskbar.rs`). Hiding Explorer's window by force was rejected: no Start
  button, and Explorer shows it again whenever it restarts.
- **We put it back.** The state from before is saved in settings (`taskbar.savedState`) the moment we change
  it, restored on switch off and on quit, and held in an atomic for the panic hook (no lock). If Barista
  crashed, the next start sees `savedState` and restores or re-applies. The setting itself stays on across
  starts. The work area changes, so the bar's strip is reserved again right after.
- **Start with Windows** is one value (`Barista`) in `HKCU\...\CurrentVersion\Run`, the quoted path of
  the running exe (`autostart.rs`). No admin and no scheduled task. The registry is the truth: Task
  Manager's Startup tab can switch it off, so the settings window reads it instead of trusting a flag. The
  default is off. A dev build writes its own path; that is what a dev build should do.

## Consequences

- A value that points at a moved exe stops counting as enabled (`is_enabled` compares paths).
