# 0009: Adaptive bar tone (dark or light content over the wallpaper)

Status: accepted (2026-11-09, `feature/design-tokens`)

## Context

On macOS (since Big Sur) the menu bar is translucent and its text and icons turn dark on a light
wallpaper and light on a dark one. Apple does not document the rule; developers who looked at it
assume it is the average brightness of the desktop picture under the bar, and that it sometimes
guesses wrong ([tyler.io](https://tyler.io/surtainly-not/),
[Apple forums](https://developer.apple.com/forums/thread/662322)). iOS calls the same idea
`lightContent` / `darkContent` status bar styles. We call the value the **tone of the wallpaper**:
`light` wallpaper -> dark content, `dark` wallpaper -> light content.

## Options

|                                | Read the wallpaper file (chosen)        | Capture the screen behind the bar      | Follow the Windows light/dark app theme |
| ------------------------------ | --------------------------------------- | -------------------------------------- | --------------------------------------- |
| Sees only the wallpaper        | yes                                     | no: includes the bar and windows over it | n/a                                   |
| Matches what macOS does        | yes                                     | no (it would flip when a window moves) | no (the theme is unrelated to the wallpaper) |
| Needs a permission or hides the bar from screenshots | no          | yes (exclude the bar from capture)     | no                                      |
| Cost                           | ~40 ms once per change (4K JPEG)        | continuous                             | none                                    |

## Decision

Rust measures how bright the wallpaper is in the strip the bar covers and sends `light | dark`.

**The maths, in plain words** (`appearance/luminance.rs`):

1. Screens store colours "bent" (sRGB gamma), so we un-bend each channel into straight light first.
2. `L = 0.2126 R + 0.7152 G + 0.0722 B` (WCAG relative luminance): eyes are most sensitive to green.
   L is 0 for black and 1 for white.
3. Average L over every pixel of the strip (light, not colour: half black and half white is "mixed").
4. Contrast between text and background is `(lighter + 0.05) / (darker + 0.05)`. Black text has contrast
   `(L + 0.05) / 0.05`, white text `1.05 / (L + 0.05)`. They are equal at `L = sqrt(1.05 * 0.05) - 0.05 = 0.179`.
   Brighter than that, black text reads better, so the wallpaper is `light`.

**Which pixels** (`appearance/layout.rs`, `platform/windows/wallpaper.rs`): `IDesktopWallpaper` gives the image
path, the placement (Fill, Fit, Stretch, Center, Tile, Span) and the background colour for the bar's monitor
(found by comparing monitor rectangles). The placement is turned into "which part of the image is under the
top `bar height` rows". WIC decodes only that part, shrunk to at most 512 px wide. Where the image does not
reach the strip (Fit bars, Center border) the background colour counts, weighted by its share of the strip.
Span is treated like Fill and Tile uses its first tile (close enough for a thin strip). No image means the
solid background colour. If the file cannot be read, `%APPDATA%\Microsoft\Windows\Themes\TranscodedWallpaper`
is tried, and if that fails the last tone stays.

**When** (no polling): one worker thread sleeps until a message arrives, waits 150 ms for bursts to settle,
measures once, and emits `bar-appearance-changed` only if the tone changed. Messages come from `WM_SETTINGCHANGE`
with `SPI_SETDESKWALLPAPER` (Settings, themes, slideshows) and from the controller after every apply (monitor, DPI
or display change moves the strip). `commands.getBarAppearance()` returns the current value.

**UI:** `shared/stores/bar-tone.ts` holds the tone and sets `<html data-tone="light|dark">`; `styles/tokens.css`
defines both token sets. The default is `light` (also the CSS default), so nothing flashes before the first answer.

**Debug:** debug builds register the global hotkey Ctrl+Alt+Shift+T (the bar never takes focus, so an in-page key
would never fire). It cycles auto, light, dark and sends the normal event.

## Consequences

- A wallpaper that is half light, half dark under the bar gets the average: it can look wrong, like on macOS.
- A "Span" wallpaper across several monitors is approximated (as Fill).
- Not watched: a wallpaper file edited in place without Windows announcing a change.
- Material You style colours from the wallpaper (a seed colour) are a later idea, see `releases/v0.8.md`.
