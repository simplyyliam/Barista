# Changelog

All notable changes to `@barista/sdk` are listed here. The SDK is versioned separately from the
Barista app (tags `sdk-vX.Y.Z`). The format follows [Keep a Changelog](https://keepachangelog.com).

## [Unreleased]

- Package set up with the protocol types generated from Rust.
- `BA.connect`, `BA.StatusBar` and `BA.StatusItem` (`create`, `setIcon`, `setLabel`, `setTooltip`,
  `setVisible`, `remove`, `on('click' | 'error')`), also exported as `BAStatusItem`, `BAStatusBar`
  and `BAConnect`. Protocol 0.2. Menus, popovers and reconnecting come later.
