// A tiny app with one status item whose label counts up every second.
// Run it with the bar running (`pnpm dev`): `pnpm test-client`. Clicks are logged here.
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

item.on("click", ({ button }) => console.log(`click (${button})`))
item.on("error", (error) => console.error(`bar refused: ${error.code}: ${error.message}`))

let count = 0
setInterval(() => item.setLabel(String(++count)), 1000)

process.on("SIGINT", () => {
  item.remove()
  bar.close()
  process.exit(0)
})
