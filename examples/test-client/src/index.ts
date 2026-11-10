// A tiny app with one status item whose label counts up every second, and a menu with every
// kind of entry. Run it with the bar running (`pnpm dev`): `pnpm test-client`.
// Clicks and menu picks are logged here.
import { readFileSync } from "node:fs"
import * as BA from "@barista/sdk"

const icon = readFileSync(new URL("../icon.svg", import.meta.url))

const started = performance.now()
let bar: BA.StatusBar
try {
  bar = await BA.connect({ appName: "Test client" })
} catch (error) {
  console.error((error as Error).message)
  process.exit(1)
}
bar.on("close", () => {
  console.log("the bar closed the connection")
  process.exit(0)
})

const item = await BA.StatusItem.create({
  id: "counter",
  icon,
  label: "0",
  tooltip: "Counts up every second",
})
console.log(
  `connected as ${bar.sessionId} (bar ${bar.barVersion}), item "${item.id}" created after ${Math.round(performance.now() - started)} ms`,
)

let count = 0
const log = (entry: BA.MenuItem, { checked }: BA.MenuClickEvent) =>
  console.log(`menu: ${entry.label ?? entry.id}${checked === undefined ? "" : ` -> ${checked}`}`)

item.setMenu([
  { id: "reset", label: "Reset counter", accelerator: "CmdOrCtrl+R", click: () => (count = 0) },
  { id: "disabled", label: "Can't click me", enabled: false },
  { type: "separator" },
  { id: "sounds", label: "Sounds", type: "checkbox", checked: false, click: log },
  { type: "separator" },
  { id: "slow", label: "Slow", type: "radio", checked: true, click: log },
  { id: "fast", label: "Fast", type: "radio", click: log },
  { type: "separator" },
  {
    id: "more",
    label: "More",
    submenu: [
      { id: "hello", label: "Say hello", click: () => console.log("menu: hello!") },
      { id: "deeper", label: "Deeper", submenu: [{ id: "bottom", label: "Bottom", click: log }] },
    ],
  },
  { role: "about", click: log },
  { role: "quit" },
])

item.on("click", ({ button }) => console.log(`click (${button})`))
item.on("error", (error) => console.error(`bar refused: ${error.code}: ${error.message}`))
bar.on("quit", () => {
  console.log("menu: quit")
  bar.close()
  process.exit(0)
})

setInterval(() => item.setLabel(String(++count)), 1000)

process.on("SIGINT", () => {
  item.remove()
  bar.close()
  process.exit(0)
})
