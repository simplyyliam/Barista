# 0012: Tray bridging with our own Shell_TrayWnd

Status: **accepted** (2026-11-14, approved by Liam; proposed 2026-11-10 from the v0.3 spike).

## Context

The bar replaces the tray, so it must see every app's tray icon on Windows 10 and 11, live, and click
them. Full comparison: `docs/research/tray-bridging.md` (local only).

## Decision

Run a hidden top-level window of class `Shell_TrayWnd` above Explorer's, so `Shell_NotifyIcon` messages
(`WM_COPYDATA` with `NIM_*`) reach us first. Read and validate each one, forward it to Explorer's tray,
and broadcast `TaskbarCreated` at start so running apps re-add their icons. Drive icons by posting their
own `uCallbackMessage` (mouse messages, version-4 anchor point) after `AllowSetForegroundWindow`.

Rejected: Win32 toolbar messages (dead on Windows 11), UI Automation (can't see overflow or hidden
icons), DLL injection into Explorer (high antivirus risk, breaks with updates).

## Consequences

- Low antivirus risk, no admin, works on both Windows versions.
- Undocumented but long-frozen message layout (32-bit handles in the payload).
- Open risk: keeping z-order priority without a fast timer. First v0.6 task: measured prototype; a 1 s
  timer is the fallback, recorded in `docs/perf/`.
- All Win32 code in `platform/windows/`, logic in `src-tauri/src/tray/` (rule 7).
