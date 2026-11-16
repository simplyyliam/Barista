// Mic mute in the bar, built only on the public SDK (v0.8 dogfood). One click mutes or unmutes
// the default microphone, the same switch Windows uses. The mic's name is in the tooltip.
// Run with the bar running: `pnpm mic-mute` (build the helper first: `cargo build -p barista-win-helper`).
import * as BA from "@baristabar/sdk"
// One import per icon: the package's index loads thousands of icons (about 100 MB of memory).
import Mic01Icon from "@hugeicons/core-free-icons/Mic01Icon"
import MicOff01Icon from "@hugeicons/core-free-icons/MicOff01Icon"
import { startHelper } from "./helper.ts"

interface MicEvent {
  type: "mic"
  present: boolean
  muted: boolean
  device: string
}

const isMicEvent = (e: unknown): e is MicEvent =>
  typeof e === "object" && e !== null && (e as { type?: unknown }).type === "mic"

const bar = await BA.connect({ appName: "Mic mute" })
bar.on("close", () => process.exit(0))
bar.on("quit", () => process.exit(0))

const item = await BA.StatusItem.create({
  id: "mic",
  icon: { icon: Mic01Icon, color: "#ffffff", height: 14 },
  tooltip: "Microphone",
})
item.on("error", (error) => console.error(`bar refused: ${error.code}`))

const helper = startHelper(
  "--mic",
  (event) => {
    if (!isMicEvent(event)) return
    // No microphone: hide the item instead of showing a button that does nothing.
    item.setVisible(event.present)
    item.setIcon({ icon: event.muted ? MicOff01Icon : Mic01Icon, color: "#ffffff", height: 14 })
    const name = event.device.trim()
    item.setTooltip(`${event.muted ? "Microphone muted" : "Microphone on"}${name ? `: ${name}` : ""}`)
  },
  () => process.exit(1),
)

item.on("click", ({ button }) => {
  if (button === "left") helper.send({ cmd: "toggle" })
})

process.once("SIGINT", () => process.exit(0))
process.on("exit", () => {
  helper.close()
  item.remove()
})
