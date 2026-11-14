# Changelog

All notable changes to `@baristabar/sdk` are listed here. The SDK is versioned separately from the
Barista app (tags `sdk-vX.Y.Z`). The format follows [Keep a Changelog](https://keepachangelog.com).

## [0.1.1] - 2026-11-14

A small fix release: a clearer hint when the bar isn't running, and a better README quick start. Speaks protocol 0.5.

### Changed

- **One waiting hint:** if the bar isn't running, a script prints a single line on stderr after about 10 seconds,
  so it doesn't look stuck. The hint is skipped when the bar answers first or after `close()`, and its timer
  never keeps your process alive.

### Fixed

- The README quick start now connects with `appName` first (`BA.connect({ appName: "Clock" })`), so the bar's
  tooltips show your app's name instead of "index".

### Known issues

- Needs Node 22 or newer.
- Windows only, and the Barista bar must be installed and running for items to show (the SDK waits if it isn't).
- Icon objects need `@resvg/resvg-js` installed; without it, use a PNG instead.
- The package is pre-1.0: names and protocol fields may still change in a minor release.

## [0.1.0] - 2026-11-14

First public release: put your app in the Barista menu bar with a status item, a menu or your own popover window.
Speaks protocol 0.5.

### Added

- **Status items:** `BA.connect()`, `BA.StatusBar` and `BA.StatusItem` with `create`, `setIcon`, `setLabel`,
  `setTooltip`, `setVisible`, `remove` and `on('click' | 'error')`.
  ```ts
  import * as BA from "@baristabar/sdk"
  const item = await BA.StatusItem.create({ id: "counter", label: "0" })
  item.on("click", () => console.log("clicked"))
  ```
- **Menus:** `BA.Menu.fromTemplate(...)` and `item.setMenu(menu)` with the Electron-style template (`label`, `type`,
  `checked`, `accelerator`, `click`, `role`, `submenu`). The bar draws the menu and runs your `click` callbacks.
- **Popovers:** `item.setPopover({ window, width, height })` shows your own window under the icon, with the
  `PopoverWindow` interface and `electronPopover` from `@baristabar/sdk/electron`. `item.on('open' | 'close')`
  work for menus and popovers, and `on('click')` carries the `anchor`.
- **Icons:** `setIcon` and `create({ icon })` take Hugeicons and Lucide icon objects or SVG text, cropped to the
  glyph and sized by height. Needs the optional peer dependency `@resvg/resvg-js`. `@baristabar/sdk/icons`
  exports `fitToGlyph`, `toPng`, `iconToSvg` and the size constants.
- **Waits for the bar and reconnects by itself:** start your app before the bar and it connects when the bar
  shows up; if the bar restarts, your items come back with their last state, menu, popover and callbacks.
  `BA.StatusBar.isAvailable()`, `bar.available` and `bar.on('available' | 'unavailable')` tell you which.
  Retries run every 100, 200, 400 ms, then every 500 ms.
- **Heartbeat and goodbye:** the SDK answers the bar's pings, so the bar can remove items of apps that died.
  `bar.close()` says goodbye and removes your items at once.
- Every name also has a `BA` prefix (`BAStatusItem`, `BAMenu`, ...), and the protocol types and
  `PROTOCOL_VERSION` are exported, generated from Rust.

### Known issues

- Needs Node 22 or newer.
- Windows only, and the Barista bar must be installed and running for items to show (the SDK waits if it isn't).
- Icon objects need `@resvg/resvg-js` installed; without it, use a PNG instead.
- The package is pre-1.0: names and protocol fields may still change in a minor release.

