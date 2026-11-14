# Quick note (Electron popover)

A pen in the bar: click it, type, click away. The note is saved to `note.txt` in
`%APPDATA%\Barista\quick-note` a moment after you stop typing, and the start of it shows
next to the icon. Right click: clear the note, quit. Esc closes the popover.

It is the smallest useful popover app: about 100 lines, using `item.setPopover` with
`electronPopover` from `@baristabar/sdk/electron`, and `item.on("open")` to reload the note each time.

```powershell
pnpm --filter @baristabar/sdk build
pnpm --filter @barista/quick-note start
```
