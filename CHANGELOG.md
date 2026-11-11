# Changelog

All notable changes to the Barista app are listed here. The SDK has its own changelog in
`packages/sdk/CHANGELOG.md` (tags `sdk-vX.Y.Z`). The format follows
[Keep a Changelog](https://keepachangelog.com).

## [Unreleased]

## [0.4.0] - 2026-11-11

Apps can now open their own small window right under their item in the bar.

### Added

- **Popovers.** Click an app's item and its window appears right below it, ready for typing. It
  works at 100%, 125% and 150% display scaling and stays on screen near the edges. Esc or a click
  anywhere else closes it, and clicking the same item again closes it too.
- **One thing open at a time.** Opening a popover closes any open menu, and the other way round.
  If an app closes or crashes while its window is open, the window goes away with it.
- **Left, right and middle click.** Left click opens the window, right click opens the menu if the
  app has one, and middle click goes to the app.
- **Apps hear when their menu or window opens or closes**, so they can refresh a label right
  before you see it.

### Changed

- **Item images keep their own shape.** Wide icons, like a battery, are no longer squeezed into a
  square; they are drawn at their own size, up to 22 px tall.
- Menus and windows now find the right place from the current screen setup, so they stay put
  after you change monitors or scaling.

### Known issues

- Windows SmartScreen may warn that the installer is from an unknown publisher, because it isn't
  code-signed yet. Click "More info", then "Run anyway".
- No automatic updates yet. To get a newer version, download its installer from the GitHub
  Releases page and run it.
- The battery and volume icons are placeholders: they don't show your real battery or volume yet.
- Keyboard shortcut hints in menus are only text. Showing "Ctrl+Enter" next to an entry doesn't
  create a real shortcut.
- An item doesn't look "pressed" while its window is open yet.
- Popovers were tested mostly on a single screen at 150% scaling; other screens and scales are
  covered by tests but have had less hands-on checking.
- The usual Windows tray icons stay in the taskbar for now; they move into the bar in a later version.
- The bar shows on one screen only, and there's no way yet to pick which screen from the app.
- Menus take about 45 ms to appear and Barista uses about 93 MB of memory, because the menu windows
  are ready in the background.

### Privacy

- Barista sends nothing over the internet. It doesn't check for updates yet.
- Apps that add items, menus or popovers talk to the bar only on this computer; nothing leaves the
  machine.
- The bar looks at the wallpaper behind it only to pick light or dark text. This happens on your
  computer, and the wallpaper is never sent anywhere.
- Tray icons aren't read yet. When they are, they'll be read locally and never sent anywhere.

## [0.3.0] - 2026-11-11

Items in the bar can now open real menus, with the mouse or the keyboard.

### Added

- **Menus under bar items.** Click an app's item and a menu drops down right below it, at 100%,
  125% and 150% display scaling. Near the edge of the screen it shifts to stay fully visible.
- **Menus with everything you'd expect:** separators, checkboxes, radio groups, disabled entries,
  keyboard shortcut hints, and submenus up to three levels deep. Ticking a checkbox or picking a
  radio entry updates right away and the app is told.
- **Use menus from the keyboard.** Arrow keys move through the entries, Right and Left open and close
  submenus, Enter picks, Home and End jump to the ends, and typing a letter jumps to the entry
  that starts with it.
- **Closes like a real menu.** Press Esc or click anywhere else to close it. Clicking another
  item's icon switches straight to its menu, and clicking the same icon again closes it.
- **Smooth submenus.** Moving the mouse diagonally toward a submenu doesn't close it on the way.
- **A frosted look** with rounded corners and a soft shadow, and the menu fades out the way
  Windows menus do.
- **Built-in entries** for Quit, About and Settings that apps can drop into their menus.
- If an app closes or crashes while its menu is open, the menu closes with it.

### Changed

- Clicking an app's item opens its menu when it has one. Apps without a menu behave as before.

### Known issues

- Windows SmartScreen may warn that the installer is from an unknown publisher, because it isn't
  code-signed yet. Click "More info", then "Run anyway".
- No automatic updates yet. To get a newer version, download its installer from the GitHub
  Releases page and run it.
- The battery and volume icons are placeholders: they don't show your real battery or volume yet.
- Keyboard shortcut hints in menus are only text. Showing "Ctrl+Enter" next to an entry doesn't
  create a real shortcut.
- Apps can't open a panel from their item yet; that comes in a later version.
- The usual Windows tray icons stay in the taskbar for now; they move into the bar in a later version.
- The bar shows on one screen only, and there's no way yet to pick which screen from the app.
- Barista uses a bit more memory than 0.2 (about 74 MB, up from 58 MB) because the menu windows
  are ready in the background so menus open instantly.

### Privacy

- Barista sends nothing over the internet. It doesn't check for updates yet.
- Apps that add items or menus talk to the bar only on this computer; nothing leaves the machine.
- The bar looks at the wallpaper behind it only to pick light or dark text. This happens on your
  computer, and the wallpaper is never sent anywhere.
- Tray icons aren't read yet. When they are, they'll be read locally and never sent anywhere.

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
