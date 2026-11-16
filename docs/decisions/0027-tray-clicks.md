# 0027: Tray clicks (ADR 0012, built in v0.7)

Status: **proposed** (2026-11-16, v0.7; becomes accepted when Liam merges the PR)

## Context

v0.6 shows every tray icon in the bar. v0.7 makes them clickable the way the real tray is: left, right,
double and middle click reach the app that owns the icon, and its own context menu opens at the icon.
The app draws that menu itself (a real Win32 menu), so the bar only has to send what Explorer sends.

## Decision

```text
UI: itemClicked("tray/<key>", button) ──► route_click ──► ClickTarget::Tray
                                                          │ registry.tray_owner(key): hwnd, id, callback, version, pid
                                                          ▼
   tray::click:  AllowSetForegroundWindow(pid) ──► PostMessage(owner hwnd, callback, wParam, lParam) × N
```

- **A run of messages, not one.** Explorer posts the app's callback message several times per click, and
  apps rely on it (some open on `WM_LBUTTONUP`, some on `NIN_SELECT`):

  | Click | Messages |
  |---|---|
  | left | `WM_MOUSEMOVE`, `WM_LBUTTONDOWN`, `WM_LBUTTONUP`, + `NIN_SELECT` (version 4) |
  | double | `WM_LBUTTONDBLCLK`, `WM_LBUTTONUP` (after the first click's normal run) |
  | right | `WM_MOUSEMOVE`, `WM_RBUTTONDOWN`, `WM_RBUTTONUP`, + `WM_CONTEXTMENU` (version 4) |
  | middle | `WM_MBUTTONDOWN`, `WM_MBUTTONUP` |

  `NIN_SELECT` and `WM_CONTEXTMENU` are only sent to icons that asked for `NOTIFYICON_VERSION_4`
  (`NIM_SETVERSION`), as Microsoft documents. **Source: Microsoft's documentation and Wine's tray code, not
  a recording of Explorer.** The Windows 11 tray buttons are not reachable with UI Automation, so the
  side-by-side recording with `scripts/tray-probe.ps1` is still to do (Liam clicks its two icons in
  Explorer's tray once; `scripts/tray-probe.ps1 -Log` prints what Explorer sends).
- **Encoding.** Version 4: `wParam = MAKEWPARAM(x, y)` (the anchor), `lParam = MAKELPARAM(message, icon id)`.
  Older: `wParam = icon id`, `lParam = message`. Coordinates are signed 16-bit words (a monitor left of the
  primary one has negative x).
- **Double click without a UI change.** The UI sends one `itemClicked(id, "left")` per click. A second left
  click on the same icon within `GetDoubleClickTime()` is the double click (`click::DoubleClick`); it uses
  up both clicks, so a third starts over. Any other button resets it.
- **Anchor.** The bottom centre of the icon's rect when the UI reported it (`setItemBounds`), else the mouse
  pointer (a tile in the tray panel has no bounds). It is converted once to the owner's DPI awareness
  (`click::to_owner`: unaware 96 dpi, system-aware system dpi, per-monitor as is). We first tried
  `PhysicalToLogicalPointForPerMonitorDPI`; it returned FALSE for these windows, so the conversion is ours.
  Checked with the probe (a DPI-unaware app on a 150 % screen): the anchor equals the app's own cursor
  position. Apps that ignore the anchor call `GetCursorPos`, and the pointer is on the icon.
- **Focus.** `AllowSetForegroundWindow(pid)` first (the pid comes from the tray message's window, never from
  the app), so the app's `SetForegroundWindow` + `TrackPopupMenu` works and its menu closes on an outside
  click. Windows only honours the grant when the bar had real input just before; a click in the UI is that.
  The debug click (`pnpm inspect --click`) is not input, so menus opened that way sometimes stay open until
  Escape or `WM_CANCELMODE` (test cleanup only).
- **Never wait on an app (rule 3).** `PostMessage`, never `SendMessage`. A hung app cannot freeze the bar. A
  hidden icon, an icon without a callback message, or a dead window is a no-op or `NotFound`.
- **Rust layout (rule 7).** Win32 in `platform/windows/tray_input.rs`; the message runs, the encoding, the
  DPI math and the double-click tracker are plain Rust in `tray/click.rs`, with unit tests.
- **`SetVersion` now reaches the registry** (`TrayOwner.version`), because it decides the encoding.

## Not done (notes in later release files)

- Hover messages (`NIN_POPUPOPEN`/`NIN_POPUPCLOSE`, rich tooltips), keyboard `NIN_KEYSELECT`.
- Double click for client items (the SDK's `item.click` has no click count).
- Right-drag, or a held-down click (we always send down and up together).

## Consequences

- Apps get the same messages as from Explorer, so no per-app code. A few apps that treat `WM_MOUSEMOVE` as
  a hover will show their hover for a moment.
- If Explorer is shown to send a different run for some app, the table in `tray/click.rs` is the one place
  to change.
