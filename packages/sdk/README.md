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

| Call                            | What it does                                                     |
| ------------------------------- | ---------------------------------------------------------------- |
| `BA.connect(options?)`          | Connects once and shares the connection. Returns a `StatusBar`.  |
| `BA.StatusItem.create(options)` | Adds an item. Options: `id`, `icon?`, `label?`, `tooltip?`.      |
| `item.setIcon(icon)`            | New icon (PNG/SVG bytes or `data:` URL). `""` removes it.        |
| `item.setLabel(text)`           | New label. `""` removes it.                                      |
| `item.setTooltip(text)`         | New tooltip. `""` removes it.                                    |
| `item.setVisible(boolean)`      | Hide or show without removing.                                   |
| `item.remove()`                 | Takes the item out of the bar.                                   |
| `item.on("click", fn)`          | `fn({ button })` when the user clicks.                           |
| `item.on("error", fn)`          | `fn(error)` when the bar refuses something about this item.      |
| `bar.on("close", fn)`           | The bar went away. Item calls do nothing after this.             |
| `bar.on("error", fn)`           | Errors that no item listener took (printed with `console.warn`). |
| `bar.close()`                   | Disconnects; the bar removes your items.                         |

Errors from the bar are `BA.BarError` objects with a `code` (`"invalid_icon"`, `"limit_reached"`, ...).

## Rules

- **Item ids**: 1 to 64 characters from `a-z 0-9 - _ .`, unique in your app.
- **At most 8 items** per app. **Labels** up to 128 characters, **tooltips** up to 512.
- **Icons**: PNG or SVG, up to 256 KB. Size them for a bar about 24 px high; the bar scales them.
- A bad argument throws right at the call (nothing is sent). Anything the bar refuses later
  arrives through `on("error")`.
- The bar draws everything. Your app never paints into it.

## Not in yet

Menus, popovers, and reconnecting after the bar restarts arrive in later releases. For now,
if the bar is not running `connect()` rejects with "Could not reach the Barista bar", and if it
quits your items disappear and the `close` event fires.
