// A focus timer in the menu bar: the label counts down, the menu starts, pauses and picks a
// preset. Run it with the bar running (`pnpm dev`): `pnpm test-client`.
// Clicks and menu picks are logged here.
import * as BA from "@barista/sdk"
import { Clock01Icon } from "@hugeicons/core-free-icons"

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
  icon: { icon: Clock01Icon, color: "#ffffff", height: 14, strokeWidth: 1.75 },
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
item.on("error", (error) => console.error(`bar refused: ${error.code}: ${error.message}`))

bar.on("quit", () => {
  console.log("quit")
  bar.close()
  process.exit(0)
})

process.on("SIGINT", () => {
  item.remove()
  bar.close()
  process.exit(0)
})