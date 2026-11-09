# 0008: Window corner radius from Windows 11 and Windhawk

Status: accepted (2026-11-09, extra on the v0.2 branch)

## Context

Windows 11 rounds window corners by 8 px and squares them when a window is maximized, snapped or
full screen. The Windhawk mod "custom-corner-radius" (it runs inside `dwm.exe`, so it changes every
window) replaces the 8 with its own radius and can optionally keep the rounding for snapped and
maximized windows. The bar and its windows should draw the same corners as everything else.

|                              | Read the registry + watch it          | Poll the registry on a timer | Ask DWM for the radius |
| ---------------------------- | ------------------------------------- | ---------------------------- | ---------------------- |
| Updates when settings change | instantly (`RegNotifyChangeKeyValue`) | up to one interval late      | no such call exists    |
| Idle cost                    | none (thread sleeps in a wait)        | wakes up forever (rule 9)    | -                      |
| Works without Windhawk       | yes (key missing = stock behaviour)   | yes                          | -                      |

## Decision

Read the mod's keys directly with the Win32 registry API and watch them with `RegNotifyChangeKeyValue`.

```text
HKLM\SOFTWARE\Windhawk\Engine\Mods\custom-corner-radius            DWORD Disabled (0 = on)
HKLM\SOFTWARE\Windhawk\Engine\Mods\custom-corner-radius\Settings   DWORD radius (-1 = keep original)
                                                                   SZ    roundMaximizedAndSnapped
```

The mod is **not active** if the key is missing, `Disabled` is not 0, `radius` is missing, or `radius` is
negative (-1 is stored as `0xFFFFFFFF`). Then the stock rules apply.

| State      | Mod not active | Mod active (radius R)                                         |
| ---------- | -------------- | ------------------------------------------------------------- |
| Normal     | 8              | R                                                             |
| Snapped    | 0              | R if the option is `snapped` or `snappedAndMaximized`, else 0 |
| Maximized  | 0              | R only if the option is `snappedAndMaximized`, else 0         |
| Fullscreen | 0              | 0, always                                                     |

- **Layout.** The rules are plain Rust in `radius.rs` (no Win32, fully unit tested). The registry code is in
  `platform/windows/windhawk.rs`, the window state (`window_state`, `is_snapped`) in `platform/windows/window.rs`,
  and `window_radius.rs` ties them to Tauri. Registry handles are RAII wrappers.
- **Watching.** One thread waits on the deepest key that exists, so it also notices Windhawk being installed
  or the mod appearing later. It arms the notification before reading (no change can slip between), and waits
  50 ms after a signal because Windhawk writes values one by one.
- **Snapped has no Windows API**, so it is decided by geometry: the visible frame
  (`DWMWA_EXTENDED_FRAME_BOUNDS`) is flush with the work area in a half, column/row or quarter pattern.
- **Per window.** Windows are registered by label with their own tracker. `Mode::Tracked` follows the real
  window (settings window); `Mode::Normal` never detects anything and always gets the normal radius (a transparent
  popover is never maximized or snapped). Each update goes only to that window, carries its `label`, and sets
  `--window-radius` on that window's own `<html>`. Updates are sent only when the value changed.
- **`customized`** is true when the value differs from what stock Windows 11 gives that state, so a mod radius
  of 8 on a normal window is not "customized". `radius` is `None` only before the first value exists.
- **Child radii** need no Rust: inner radius = parent radius minus padding, in CSS
  `max(0px, calc(var(--window-radius) - 12px))`.

## Consequences

- The radius is in logical pixels, taken as the mod stores it. If Windhawk later stores physical pixels, only
  `settings_from_values` changes.
- Windows 10 has no rounded corners; the bar still reports 8 there. Revisit if Windows 10 matters.
- Only the registry watcher thread and window events touch the state; one lock covers sending, so updates
  never arrive out of order.
- **Testing hook.** Changing HKLM needs admin, so in debug builds `BARISTA_WINDHAWK_HKCU_KEY` (a key under HKCU) replaces
  the HKLM location. Release builds do not contain it. With it the real bar was driven live against a changing key.
- **Test windows.** Debug builds with `BARISTA_RADIUS_TEST_WINDOWS` open a tracked "settings" window and a normal
  "popover" window so the per-window behaviour could be exercised with real maximize, snap and fullscreen. Extra
  windows must repeat the main window's `additionalBrowserArgs`, or WebView2 fails to start them.
