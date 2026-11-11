# 0014: Popovers are the app's own window; the bar decides when, where and when to hide

Status: accepted (2026-11-11, v0.4 Popovers)

## Context

Menus are drawn by the bar. A mini player or a note box needs the app's own UI. The research
(`docs/research/sdk-presentation-architecture.md`) compared a window owned by the app (A) with UI
hosted by Barista (B). A keeps third-party page code out of the bar's renderer (rule 1, rule 3, the
memory budget); B can come later as an SDK helper on the same messages.

## Decision

**Anchoring: the bar places, the app applies.**
- `item.setPopover { width, height }` (logical px, 48 to 1024; `0 x 0` removes). It is a request.
- On a left click the bar computes the final rect with `anchor::place` (under the icon, shifted to
  fit, 10 px margin from the left, right and bottom screen edges, height capped to the work area).
  The size is converted to physical pixels once. The bar sends it in `popover.show { rect, scale,
  monitor, tone }`. Rects are physical px in virtual-desktop coordinates (a monitor left of the primary
  one has negative `x`). The app moves its window there as is; the SDK adapters convert to the
  framework's pixels (Electron: `screen.screenToDipRect`).
- `item.click` carries `anchor` (the item's rect) for apps that place their own window.
- Items without bounds from the UI fall back to the mouse x (same as menus, ADR 0010).

**The click rule** (one place: `menu_window::route_click`, using `Registry::target`):

| Button | Item has popover | Else menu | Else |
|---|---|---|---|
| left | popover | menu | `item.click` |
| right | `item.click` | menu | `item.click` |
| middle | `item.click` | `item.click` | `item.click` |

An item may have both: left opens the popover, right opens the menu. Middle always reaches the app,
so apps keep one free gesture. A click on any item closes a popover that belongs to another item.

**One thing open at a time.** Opening a popover closes the menu and the other way round. The popover
shares the menu's open/close rules: `dismiss::Machine` (toggle on the same icon, swap on another,
`BLUR_GRACE` of 150 ms so the click that caused a blur decides) was extracted from `menu_window.rs`.

**Focus.** Windows only lets a program take focus if it was just allowed to. Right before
`popover.show` the bar calls `AllowSetForegroundWindow(pid)`. The pid comes from the named pipe
(`GetNamedPipeClientProcessId`, already checked against `hello`), never from the client's word.
The grant only works if the bar just received real input, which a user click is; a click faked over the pipe
(`debug.click`) shows the window but does not focus it (tested with real mouse clicks instead).

**Hiding.** The bar sends `popover.hide` when: the same icon is clicked, another item is clicked, another
program becomes the foreground (after the grace), the item or its popover goes away, or the app never
confirmed. "Another program is in front" comes from `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`, an event
and not polling (rule 9). It is ignored until the app confirmed with `popover.shown`, because
the window may not be up yet; the timeout covers that case.

**Timeout.** After `popover.show` the app has 1.5 s to answer `popover.shown` (the SDK sends it once the
window's `show` returned). If it does not, the bar hides the popover and the item goes back to idle: no stuck
"open" state. A dead client's items are removed with its session, which closes the popover.

**Who answers what.** `popover.closed` means "I closed it myself" (Esc, window destroyed). The app must
NOT answer a `popover.hide` with `closed`: a late `closed` after a quick re-open would close the new popover.
The SDK enforces this (it tracks whether the bar opened the popover).

**Esc is the app's job.** The bar cannot see keys typed into another program's window without a global
keyboard hook, and a hook that steals Esc would break apps that use it. The window has focus, so it sees Esc: the
Electron adapter hides the window on Esc and reports `popover.closed`. Another framework implements
`PopoverWindow.onClose` the same way. (The v0.4 goal text says Esc "sends popover.hide"; in this design the
result is the same for the user and the bar is told with `closed`.)

**The UI.** `popoverChanged { itemId | null }` and `getOpenPopover()` let the UI show the item as open.

**`menu.opened` / `menu.closed`** go to the owner of a menu, so `item.on("open" | "close")` works for menus too
(an app can refresh a label right before its menu shows).

**The SDK stays framework-free.** `PopoverWindow { show(placement), hide(), onClose(listener) }` is a
structural interface. `@barista/sdk/electron` takes a `BrowserWindow` and `screen` as arguments and imports nothing
from Electron; lint forbids importing `electron` anywhere in `packages/`. Electron is a dev dependency of the
Electron examples only. A Tauri adapter moves to the v1.2 Rust crate and plugin: the SDK is Node-only and
cannot run in a webview.

## Consequences

- Barista is not a sandbox. A popover app is a normal program running as the user. The bar only ever shows
  it after a user click, never on request, and caps its size.
- Protocol 0.4 is additive (same major): a 0.3 client keeps working, and an older bar answers `unknown_type` to
  `item.setPopover`.
- Placement is physical-pixel math tested per scale (100, 125, 150, 200 %) and with negative monitor coordinates.
- Windows draws an invisible frame around some windows; Electron frameless windows measure about 7 DIP bigger than
  the visible rect with `GetWindowRect`. The visible window lands on the rect.
- Not decided here: popover hosted by Barista (backlog), reconnect re-sending `item.setPopover` (v0.5).
