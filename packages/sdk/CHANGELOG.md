# Changelog

All notable changes to `@baristabar/sdk` are listed here. The SDK is versioned separately from the
Barista app (tags `sdk-vX.Y.Z`). The format follows [Keep a Changelog](https://keepachangelog.com).

## [Unreleased]

- Package set up with the protocol types generated from Rust.
- Icon objects: `setIcon` and `create({ icon })` take Hugeicons and Lucide icon objects (and SVG text), cropped to
  their glyph and sized by height with `@resvg/resvg-js` (optional peer dependency). `@baristabar/sdk/icons` exports
  `fitToGlyph`, `toPng`, `iconToSvg` and the size constants.
- `item.setPopover({ window, width, height })` with the `PopoverWindow` interface, `@baristabar/sdk/electron`
  (`electronPopover`), `item.on('open' | 'close')` for menus and popovers, and `anchor` on `on('click')`.
  Protocol 0.4.
- `BA.connect`, `BA.StatusBar` and `BA.StatusItem` (`create`, `setIcon`, `setLabel`, `setTooltip`,
  `setVisible`, `remove`, `on('click' | 'error')`), also exported as `BAStatusItem`, `BAStatusBar`
  and `BAConnect`. Protocol 0.2. Menus, popovers and reconnecting come later.
