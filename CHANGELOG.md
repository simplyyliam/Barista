# Changelog

All notable changes to the Barista app are listed here. The SDK has its own changelog in
`packages/sdk/CHANGELOG.md` (tags `sdk-vX.Y.Z`). The format follows
[Keep a Changelog](https://keepachangelog.com).

## [Unreleased]

## [0.2.0] - 2026-11-09

The first Barista release: a macOS-style menu bar at the top of your screen that apps can add their own items to.

### Added

- **A menu bar across the top of your screen.** It reserves its own space, so maximized windows
  stop just under it instead of sliding behind it. Works at 100%, 125% and 150% display scaling.
- **Your screen space comes back when Barista closes**, even if it crashes or is ended from Task Manager.
- **Gets out of the way for full-screen apps.** Videos and games in full screen hide the bar; it
  comes back when you leave full screen.
- **Apps can add their own items to the bar**, with an icon, a short label and a tooltip. Items
  update live, and clicking one tells the app it was clicked. When an app closes or crashes, its
  item disappears by itself.
- **A clock** on the right side of the bar.
- **Readable on any wallpaper.** The bar's text switches between light and dark to match the
  wallpaper behind it, including when you change the wallpaper in Settings.
- If the display setup changes (a monitor unplugged, scaling changed), the bar moves and resizes itself.

### Known issues

- Windows SmartScreen may warn that the installer is from an unknown publisher, because it isn't
  code-signed yet. Click "More info", then "Run anyway".
- No automatic updates yet. To get a newer version, download its installer from the GitHub
  Releases page and run it.
- The battery and volume icons are placeholders: they don't show your real battery or volume yet.
- Clicking an app's item doesn't open a menu or panel yet; that comes in a later version.
- The usual Windows tray icons stay in the taskbar for now; they move into the bar in a later version.
- The bar shows on one screen only, and there's no way yet to pick which screen from the app.

### Privacy

- Barista sends nothing over the internet. It doesn't check for updates yet.
- Apps that add items talk to the bar only on this computer; nothing leaves the machine.
- The bar looks at the wallpaper behind it only to pick light or dark text. This happens on your
  computer, and the wallpaper is never sent anywhere.
- Tray icons aren't read yet. When they are, they'll be read locally and never sent anywhere.
