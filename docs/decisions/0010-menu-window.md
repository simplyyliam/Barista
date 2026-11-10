# 0010: One reused menu window, placed by Rust

Status: accepted (2026-11-10, v0.3 Menus)

## Context

Menus drop below the bar, so they can't live in the bar window. We need a window for them that opens
fast, sits under the right icon at every DPI scale and near screen edges, closes like a real menu, and
keeps placement logic in Rust (rule 5). Liam draws the menu in React and wants his own corners and shadow.

## Decision

**One window, made at startup, hidden, reused.** Label `menu`, transparent, undecorated, no Windows
shadow, square DWM corners (`DWMWCP_DONOTROUND`), tool window (not in Alt+Tab), always on top. Opening a
menu = resize, move, show, focus; closing = hide. No webview start-up on each open.

| | One reused window | A window per open | Full-screen overlay |
|---|---|---|---|
| Open speed | instant | 100-300 ms webview start | instant |
| Memory | one webview, always | spikes | one big transparent webview |
| Outside click | blur closes | blur closes | overlay eats the click |
| Edge flip in Rust | yes | yes | partly in the UI |

**Size comes from the UI, placement from Rust.** The window first shows at a size guessed from the
entries. The UI measures itself and calls `menuResized(width, height, rootWidth)` (logical px). Rust
converts once to physical pixels and runs `registry::anchor::place`: top edge on the bar's bottom,
left edge on the item's left, shifted left to stay in the work area, height capped. Submenus render in
the same window, which grows sideways; if the submenu doesn't fit on the right it flips left and the
main panel stays under the icon. The reply tells the UI which side.

**Where the icon is.** The UI sends item rects with `setItemBounds` (logical, bar-relative). Without
them the menu opens under the mouse (`GetCursorPos`), clamped to the bar.

**Closing.** A small state machine (`menu_window::Machine`, unit tested):

- Esc / a pick / the app's menu disappears → close.
- Click the same icon again → close. Click another icon → swap contents, no hide (no flicker).
- Window loses focus → hide after 150 ms, unless a click on an icon arrives first. If the blur closed
  a menu and the click lands on that same icon right after, it stays closed (the user meant "close").

## Consequences

- One extra webview at all times (bench in the v0.3 PR). Its renderer shares the process limit.
- `menu-open`, `menu-changed`, `menu-close` go to every window; the menu window draws, the bar can
  highlight the open item.
- The bar window has `WS_EX_NOACTIVATE`, so clicking it doesn't take focus from the menu; the blur
  grace is only a safety net for the order of events.
- Popovers (v0.4) can reuse `anchor::place` and the item bounds.
