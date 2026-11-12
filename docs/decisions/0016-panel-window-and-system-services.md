# 0016: One reused panel window, and system services with one shape

Status: accepted (2026-11-12, v0.4b slice A)

## Context

The design has built-in items (clock, battery, volume) and panels that hang under them (control
center, notifications, calendar). They are not client apps, so they use neither the popover protocol
(ADR 0014) nor the menu window (ADR 0010). The bar window is a 32 px strip, so a panel needs a
window of its own, and the frontend may only render state it gets from Rust (rule 5).

## Decision: the panel window (`panel.rs`)

**One window, made at startup, hidden, reused.** Label `panel`, same styles as the menu window
(transparent, no decorations, DWM rounded corners and acrylic, tool window, always on top), same
`additionalBrowserArgs` as `tauri.conf.json`, registered with `window_radius::register(.., Mode::Normal)`
(ADR 0008). It loads `index.html`; the UI picks the route from the `name` in `panelChanged`
(`control-center`, `notifications`, ...). One WebView2 per panel would eat the memory budget.

```text
UI: openPanel(name, anchor, w, h) -> Machine (toggle same anchor / swap another)
    -> close menu + popover, place under the anchor, emit panelChanged
UI renders the route, measures -> panelResized -> place again, show + focus (first time)
close: closePanel | same anchor again | focus lost (150 ms grace) | menu, popover or other item clicked
```

- **Placement is reused, not copied.** `placement::item_rect` (the anchor's rect from `setItemBounds`,
  mouse x as fallback), `menu_window::place_logical` -> `anchor::place` (physical pixels, work-area
  clamp, 10 px margin), `menu_window::{build_window, move_to, frame_extra, transitions}` (now
  `pub(crate)`). A panel is "a menu with another window", so it shares the placement tests.
- **Open/close rules are the shared `dismiss::Machine`.** Anchors are the machine's keys. One thing is open
  in the whole bar: opening a panel closes the menu and the popover; `route_click` (all three branches)
  and the popover's `close_menu` closure close the panel.
- **Focus loss uses the window's own `Focused` event**, like the menu (it is our window). The foreground
  hook (`platform/windows/foreground.rs`) stays what it is: the tool for windows that belong to another
  program (popovers).
- **Esc is the UI's job** (`closePanel`), the same as `closeMenu`: the window has focus, so the page sees
  the key; a global hook would steal Esc from other apps.
- **Size comes from the UI, placement from Rust.** `openPanel` carries a first guess; the window is shown
  when `panelResized` arrives, or after 30 ms if the UI never answers.
- Names are `[a-z0-9-]`, 32 chars at most; anchors 256; sizes go through `clean_size`.
- Clicking the same anchor with another `name` closes the panel (the anchor is the toggle key).

## Decision: system services (`system/`)

Every service has the same four parts, so the UI treats them the same way:

| Part | Clock | Battery | Volume |
|---|---|---|---|
| Snapshot (`getX`) | `getClockSettings` | `getBattery` | `getVolume` |
| Change event (full snapshot, one per frame at most) | `clockSettingsChanged` | `batteryChanged` | `volumeChanged` |
| Commands that change the system | none | none | `setVolume`, `setMuted` |
| What wakes us | `WM_TIMECHANGE`, `WM_SETTINGCHANGE` ("intl") | `WM_POWERBROADCAST` (`RegisterPowerSettingNotification` for charge, power source, battery saver) | Core Audio callbacks (`IAudioEndpointVolumeCallback`, `IMMNotificationClient`) |

- **`Service<T>`** (`system/mod.rs`) wraps a `tokio::sync::watch` channel. `set` only wakes anyone if the
  value really changed (`send_if_modified`); `forward` waits one frame (16 ms) after a change and emits
  the latest snapshot, like `items-changed`. Nothing polls: the forward task sleeps in `changed().await`.
- **Windows messages come through the bar window's existing subclass** (`window.rs`) as new `WindowEvent`
  variants, so no extra hidden window is needed. A notification only says "something changed"; the service
  reads the full state again (no partial updates to get wrong).
- **Volume lives on one COM thread** (`platform/windows/audio.rs`). Windows calls our callbacks on its own
  threads, and calling COM from inside a callback can deadlock, so the callbacks only send a message to the
  owner thread, which re-reads level and mute. A default-device change re-binds the endpoint.
  `setVolume` clamps to 0..1 and refuses NaN.
- **Battery:** `present: false` on a desktop (flag 128) or an unknown status (255). Unknown numbers (255,
  `u32::MAX`) become `None`, never a fake 0.
- **Clock:** 12 or 24 hour comes from the long time pattern (`H` = 24 h, `h` = 12 h, quoted text skipped),
  the first weekday from `LOCALE_IFIRSTDAYOFWEEK` (converted to JS numbering, 0 = Sunday), the zone from
  `GetDynamicTimeZoneInformation`. The time itself stays in JS.
- **Windows-only code** sits in `platform/windows/{locale,power,audio}.rs` (rule 7); the services read like
  plain Rust and their mapping logic is unit tested.

## What is not possible, and what the UI shows

- The Windows time zone id (`W. Europe Standard Time`) is not an IANA name. JS needs the UI to use the
  offset (`utcOffsetMinutes`); an IANA name needs a mapping table or ICU. Noted for later.
- Per-app volume and output device switching are slice B.

## Consequences

- One extra WebView2 window at all times, like the menu (measured in `docs/perf/0.4b.md`).
- `windows-core` is now a direct dependency (the `#[implement]` macro expands to code that names it); same
  version as `windows` and Tauri, so Cargo builds one copy.
- Debug builds get `debug.system`, `debug.openPanel`, `debug.closePanel` (and `pnpm inspect --system`,
  `--panel`) to show real values and open a panel without the UI. Protocol stays 0.4 (additive).
