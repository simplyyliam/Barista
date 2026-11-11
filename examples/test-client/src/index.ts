// A focus timer in the menu bar: the label counts down, the menu starts, pauses and picks a
// preset. It uses every kind of menu entry (normal, disabled, checkbox, radio, separator,
// submenu, roles). Run it with the bar running (`pnpm dev`): `pnpm test-client`.
// Clicks and menu picks are logged here.
//
// It has a second icon with a popover: a left click shows the client's own window under it.
// Plain `pnpm test-client` uses a fake window that only logs; `pnpm test-client:electron` opens
// a real one (Electron), which is what the v0.4 showcase uses.
import { readFileSync } from "node:fs"
import * as BA from "@barista/sdk"
import { createNotePopover } from "./note-popover.ts"

const icon = readFileSync(new URL("../icon.svg", import.meta.url))

const started = performance.now()
let bar: BA.StatusBar
try {
  bar = await BA.connect({ appName: "Focus" })
} catch (error) {
  console.error((error as Error).message)
  process.exit(1)
}
bar.on("close", () => {
  console.log("the bar closed the connection")
  process.exit(0)
})

const item = await BA.StatusItem.create({
  id: "timer",
  icon,
  label: "25:00",
  tooltip: "Focus timer",
})
console.log(
  `connected as ${bar.sessionId} (bar ${bar.barVersion}), item "${item.id}" created after ${Math.round(performance.now() - started)} ms`,
)

let minutes = 25
let left = minutes * 60
let ticker: NodeJS.Timeout | undefined
let sounds = true

const clock = (s: number) =>
  `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`

function tick() {
  left -= 1
  item.setLabel(clock(left))
  if (left > 0) return
  stop()
  left = minutes * 60
  console.log(sounds ? "done! (ding)" : "done!")
  item.setLabel(clock(left))
}

function stop() {
  clearInterval(ticker)
  ticker = undefined
  showMenu()
}

const preset = (m: number): BA.MenuItemOptions => ({
  id: `p${m}`,
  label: `${m} minutes`,
  type: "radio",
  checked: minutes === m,
  click: () => {
    minutes = m
    console.log(`preset: ${m} min`)
    if (!ticker) {
      left = m * 60
      item.setLabel(clock(left))
    }
  },
})

// The menu changes with the timer state (Start/Pause enabled), so it is rebuilt each time.
function showMenu() {
  const running = ticker !== undefined
  item.setMenu([
    {
      id: "start",
      label: left < minutes * 60 && !running ? "Resume" : "Start",
      accelerator: "CmdOrCtrl+Enter",
      enabled: !running,
      click: () => {
        console.log("start")
        ticker = setInterval(tick, 1000)
        showMenu()
      },
    },
    {
      id: "pause",
      label: "Pause",
      enabled: running,
      click: () => {
        console.log("pause")
        stop()
      },
    },
    {
      id: "reset",
      label: "Reset",
      click: () => {
        console.log("reset")
        left = minutes * 60
        item.setLabel(clock(left))
        stop()
      },
    },
    { type: "separator" },
    { id: "presets", label: "Presets", submenu: [preset(15), preset(25), preset(50)] },
    {
      id: "sounds",
      label: "Sounds",
      type: "checkbox",
      checked: sounds,
      click: (_, { checked }) => {
        sounds = checked ?? !sounds
        console.log(`sounds: ${sounds ? "on" : "off"}`)
      },
    },
    { role: "about", click: () => console.log("Focus 1.0, a Barista example") },
    { role: "quit" },
  ])
}
showMenu()

item.on("click", ({ button }) => console.log(`click (${button})`))

const note = await BA.StatusItem.create({
  id: "note",
  icon: readFileSync(new URL("../note.svg", import.meta.url)),
  tooltip: "Popover",
})
note.setPopover({ window: createNotePopover(), width: 320, height: 240 })
note.setMenu([{ role: "quit" }]) // right click
note.on("open", () => console.log("note: open"))
note.on("close", () => console.log("note: close"))
note.on("click", ({ button, anchor }) => console.log(`note: click (${button})`, anchor ?? ""))
note.on("error", (error) => console.error(`bar refused: ${error.code}: ${error.message}`))
item.on("error", (error) => console.error(`bar refused: ${error.code}: ${error.message}`))
bar.on("quit", () => {
  console.log("quit")
  bar.close()
  process.exit(0)
})

process.on("SIGINT", () => {
  note.remove()
  item.remove()
  bar.close()
  process.exit(0)
})
