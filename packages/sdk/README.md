# @barista/sdk

Put your app in the Barista menu bar for Windows.

```ts
import * as BA from "@barista/sdk"

const item = await BA.StatusItem.create({ id: "counter", label: "0" })
item.on("click", ({ button }) => console.log("clicked with", button))
```

Needs Node 22 or newer and the Barista bar running. Every name is also exported with a `BA`
prefix (`import { BAStatusItem } from "@barista/sdk"`).

## Quick start

```ts
import { readFileSync } from "node:fs"
import * as BA from "@barista/sdk"

// 1. Connect (optional: StatusItem.create does it for you, but this lets you name your app
//    and react when the bar goes away).
const bar = await BA.connect({ appName: "Clock" })
bar.on("close", () => process.exit(0))

// 2. Create an item. The icon is PNG or SVG bytes, the label is short text.
const item = await BA.StatusItem.create({
  id: "time",
  icon: readFileSync("clock.svg"),
  label: "12:00",
  tooltip: "The time",
})

// 3. Change it whenever you like.
setInterval(() => item.setLabel(new Date().toLocaleTimeString()), 1000)

// 4. React to clicks.
item.on("click", ({ button }) => console.log(button)) // "left" | "right" | "middle"
```

## API

| Call                            | What it does                                                          |
| ------------------------------- | --------------------------------------------------------------------- |
| `BA.connect(options?)`          | Connects once and shares the connection. Returns a `StatusBar`.       |
| `BA.StatusItem.create(options)` | Adds an item. Options: `id`, `icon?`, `label?`, `tooltip?`.           |
| `item.setIcon(icon)`            | New icon (PNG/SVG bytes or `data:` URL). `""` removes it.             |
| `item.setLabel(text)`           | New label. `""` removes it.                                           |
| `item.setTooltip(text)`         | New tooltip. `""` removes it.                                         |
| `item.setVisible(boolean)`      | Hide or show without removing.                                        |
| `item.remove()`                 | Takes the item out of the bar.                                        |
| `item.setMenu(menu \| null)`    | Gives the item a menu (a `BA.Menu` or a template). `null` removes it. |
| `BA.Menu.fromTemplate(t)`       | Builds and checks a menu. `menu.getItemById(id)` finds an entry.      |
| `item.setPopover(opts \| null)`  | Gives the item a popover: your own window under the icon.             |
| `item.on("click", fn)`          | `fn({ button, anchor? })` when the user clicks (middle click, or no menu or popover). |
| `item.on("open" \| "close", fn)` | Right before a menu or popover shows, and when it closes.            |
| `item.on("error", fn)`          | `fn(error)` when the bar refuses something about this item.           |
| `bar.on("close", fn)`           | The bar went away. Item calls do nothing after this.                  |
| `bar.on("error", fn)`           | Errors that no item listener took (printed with `console.warn`).      |
| `bar.close()`                   | Disconnects; the bar removes your items.                              |
| `bar.on("quit", fn)`            | The user picked a `role: "quit"` entry (else the SDK exits).          |

Errors from the bar are `BA.BarError` objects with a `code` (`"invalid_icon"`, `"limit_reached"`, ...).

## Menus

Clicking an item with a menu opens the menu under it (a middle click still fires `on("click")`).
The template is Electron's:

```ts
item.setMenu([
  { label: "Start", accelerator: "Ctrl+S", click: () => timer.start() },
  {
    id: "sounds",
    label: "Sounds",
    type: "checkbox",
    checked: true,
    click: (entry, { checked }) => console.log("sounds", checked),
  },
  {
    label: "Presets",
    submenu: [
      { id: "p25", label: "25 min", type: "radio", checked: true },
      { id: "p50", label: "50 min", type: "radio" },
    ],
  },
  { type: "separator" },
  { role: "about" },
  { role: "quit" },
])
```

- Types: `normal`, `checkbox`, `radio` (adjacent radios form a group), `separator`, `submenu`.
- The bar flips checkboxes and radios itself and tells you the new `checked`.
- Roles `about` and `settings` go to the top, `quit` to the bottom, labelled for you if you
  leave out `label`. A `quit` entry without `click` removes your items and ends the process,
  unless you listen to `bar.on("quit")`.
- `accelerator` is shown as text only (`CmdOrCtrl` becomes `Ctrl`); it is not a hotkey.
- At most 3 levels deep and 50 entries in total; ids unique. Changed an entry? Call
  `item.setMenu(menu)` again.

## Icons

Use an icon from Hugeicons or Lucide directly, no SVG file needed:

```ts
import { Clock01Icon } from "@hugeicons/core-free-icons" // or: import { Clock } from "lucide"
const item = await BA.StatusItem.create({ id: "time", icon: Clock01Icon })
item.setIcon({ icon: Clock01Icon, color: "#8b95a5", strokeWidth: 1.75 })
```

- `setIcon` / `create({ icon })` take an icon object, `{ icon, color?, strokeWidth?, height? }`, SVG text,
  PNG/SVG bytes or a `data:` URL.
- Icon sets draw with an empty border, so a plain 16 px icon looks small. Icon objects are cropped to their real
  glyph and sized by height (16 px square, `height: BA.WIDE_ICON_HEIGHT` for wide shapes) when you install
  `@resvg/resvg-js` (`npm install @resvg/resvg-js`). Without it they still work, just smaller.
- For your own SVG: `import { fitToGlyph } from "@barista/sdk/icons"`, then `item.setIcon(fitToGlyph(svg, 16))`.
  `toPng(svg, { height, scale })` makes a PNG at `height * scale` pixels. The sizes (`ICON_SIZE`, `WIDE_ICON_HEIGHT`,
  `ICON_AREA`) are exported from the same place.

## Popovers

A popover is your own window, shown right under the icon. The bar decides when (a left click) and
where (it sends the final rect); your app only moves and shows its window.

```ts
import * as BA from "@barista/sdk"
import { electronPopover } from "@barista/sdk/electron"
import { BrowserWindow, screen } from "electron"

const win = new BrowserWindow({ show: false, frame: false, skipTaskbar: true, alwaysOnTop: true })
const item = await BA.StatusItem.create({ id: "note", label: "Note" })
item.setPopover({ window: electronPopover(win, { screen }), width: 320, height: 380 })
item.setMenu([{ role: "quit" }]) // right click
```

- Click rule: **left** opens the popover (else the menu, else `on("click")`), **right** opens the menu
  (else `on("click")`), **middle** always fires `on("click")`.
- The bar hides the popover when the user clicks the icon again, another item, or anywhere else.
  Esc is handled by your window: the Electron adapter closes it on Esc.
- Do not hide the window on blur yourself; the bar does it, after a short grace.
- Any other window framework: pass a `BA.PopoverWindow` (`show(placement)`, `hide()`, `onClose(fn)`).
  `placement` is in physical pixels, already placed and kept on screen.
- If your window never confirms (the SDK confirms after `show` returns), the bar hides it after 1.5 s.
- The SDK never imports Electron; `electronPopover` takes your window and `screen`.

## Rules

- **Item ids**: 1 to 64 characters from `a-z 0-9 - _ .`, unique in your app.
- **At most 8 items** per app. **Labels** up to 128 characters, **tooltips** up to 512.
- **Icons**: an icon object, PNG (at most 1024x1024 pixels) or SVG, up to 256 KB. The bar is 24 px high and icons stay within 22: see "Icons".
- A bad argument throws right at the call (nothing is sent). Anything the bar refuses later
  arrives through `on("error")`.
- The bar draws everything. Your app never paints into it.

## Not in yet

Reconnecting after the bar restarts arrives in a later release. For now,
if the bar is not running `connect()` rejects with "Could not reach the Barista bar", and if it
quits your items disappear and the `close` event fires.
