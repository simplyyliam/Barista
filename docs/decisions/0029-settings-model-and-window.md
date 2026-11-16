# 0029: Settings model and window

Status: **proposed** (2026-11-16, v0.8; becomes accepted when Liam merges the PR)

## Context

v0.8 adds the settings window (`docs/design/settings-window.png`). Its sidebar has General, Menu bar,
Dock, Sidedock, Connected Apps, Script Items, Script Gallery, Advanced, Developer and About, and its big
picture is an editor: the menu bar and the control center can be dragged, trimmed and extended in place;
the options for everything else sit under "Appearance". Later releases must add options to an existing
section instead of a new window. Rule 5: Rust holds the logic, the UI only draws state.

## Decision

- **Settings are grouped like the sidebar** (`settings.rs`): `general` (monitor, start with Windows),
  `menuBar` (height, appearance, items, clock, taskbar, tray, layout), `controlCenter` (tiles with size and
  visibility, user card, sliders, now playing, quick actions), `behavior`, `advanced` (the
  `barista | windows` switches of rule 11), plus the unread stamp. A v0.7 flat file is migrated on load
  (`migrate`), so nothing is lost.
- **Rust validates everything** (rule 3): unknown ids dropped, duplicates removed, missing ids appended with
  their defaults, numbers clamped, the font name restricted to letters, digits, space, `.-_` (it ends up in
  CSS). Imported files go through the same path.
- **One event, whole tree.** Every command saves and emits `settingsChanged(Settings)`. The UI keeps one
  store per window and redraws from it; no field has its own event. The older commands (`setBarHeight`,
  `setMonitor`, the three mode switches) stay and also emit it.
- **Commands per section**, not per field: `setAppearance`, `setMenuBarItems`, `setClockPrefs`,
  `setControlCenterPrefs`, `setBehavior`, `setTrayPrefs`, `setAdvanced`, `setTaskbarAutoHide`,
  `setStartWithWindows`, `resetSettings(section)`, `exportSettings` / `importSettings`.
- **Bar height flips** (the v0.8 note): Rust stores it, the UI writes `--menubar-height` from it
  (`useApplySettings`). The UI no longer pushes its CSS value to Rust.
- **The editor is real UI, not a picture.** The preview draws the real bar icons and the saved clock
  format on the wallpaper; items move with pointer events (Tauri turns native drag and drop into file
  drops, so HTML5 drag does not work). Dragging an item off the bar hides it (macOS does the same).
  Without a mouse: Alt+Left / Alt+Right move the focused item, Delete removes it, move buttons in the
  inspector, and a live region announces the change (WCAG 2.5.7). While dragging, listeners sit on `window`:
  React moves the dragged element when the order changes, and a moved element loses pointer capture.
- **The settings window** is a normal decorated window (`settings_window.rs`, label `settings`, built on its
  own thread like the debug window), `window_radius` mode `Tracked` (it can be maximized).
- **Appearance:** `tone` overrides the wallpaper sampling of ADR 0009 (`auto` follows it); `blur` also
  checks Windows' transparency setting and battery saver every time a menu or panel opens (no watcher);
  font, size, solid background and icon style are CSS variables and attributes.
- **Not in v0.8:** wallpaper-seeded palette (Material Color Utilities) and real blur behind the bar go to
  `releases/v1.0.md` as notes (they need an evaluation and DWM work).

## Consequences

- A new option is one field in `settings.rs`, one clamp, one control on a page. No new command or event.
- Settings written by a newer Barista are read by an older one with unknown fields ignored (serde default).
