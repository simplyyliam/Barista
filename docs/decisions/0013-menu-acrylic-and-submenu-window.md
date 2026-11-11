# 0013: Real acrylic, so the submenu gets its own window

Status: accepted (2026-11-11, v0.3 menu polish). Amends 0010.

## Context

Liam wants menus that look like macOS ones: a blurred, tinted panel with round corners and a soft
shadow. CSS `backdrop-filter` only sees our own page, so real blur has to come from Windows (DWM
acrylic). DWM blur covers a whole window rectangle, and in 0010 the root panel and its submenu share one
window, so the blur would also fill the gap and the empty corners.

## Decision

**Two reused windows**, both made at startup and hidden: `menu` (main panel) and `menu-sub` (submenu).
Each is exactly one panel.

- Both get DWM acrylic (`DWMSBT_TRANSIENTWINDOW`), round corners, the DWM shadow, and no Windows
  show/hide animation. Dark or light tint follows the bar tone.
- `menu-sub` is `WS_EX_NOACTIVATE`: it never takes focus, so the keyboard stays in `menu`. Rust relays
  the highlight between the windows (`submenuHighlight` / `submenuHovered`), so mouse and keyboard
  still share one highlight.
- Open: Rust places the window, emits `menuOpen`, and shows it only when the UI reports its size
  (`menuResized`, measured before paint), so no stale frame. An 80 ms fallback shows it anyway.
- Close: `menuClose` goes out and both windows hide at once. Windows' own hide animation is switched
  on only for that moment, so glass and shadow fade out together like a native menu.
- Hover-to-switch: `menuHover(itemId)` swaps menus only while one is open. It never opens one.
- Windows 10 / older Win11 have no system backdrop: `MenuAnchor.acrylic = false`, the UI draws an opaque panel.

## Consequences

- The glass can't fade from CSS. A first version faded the content and hid the window 170 ms later,
  which left a blank ghost panel and felt laggy (Liam, 2026-11-11). So there is no CSS fade; the
  fade is Windows' own, and the `motion` package is installed but unused by the menu.
- A second WebView2 window costs memory (measured in `docs/perf/0.3.md`).
- Placement lives in `registry/anchor.rs::place_submenu` (pure, tested): right side, flips left at
  the screen edge, stays inside the work area.
- Not changed: SDK and protocol.
