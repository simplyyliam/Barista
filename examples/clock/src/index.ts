// Dogfooding client: written from the SDK README only (no source reading).
// A clock in the bar. Left click switches 12h/24h, right click hides the seconds.
import * as BA from "@barista/sdk"
import { Clock01Icon } from "@hugeicons/core-free-icons"


const bar = await BA.connect({ appName: "Clock" })
bar.on("close", () => process.exit(0))

const item = await BA.StatusItem.create({
  id: "time",
  icon: { icon: Clock01Icon, color: "#8b95a5" }, // an icon object: no SVG needed
  label: "--:--",
  tooltip: "Click to switch 12h / 24h",
})

let twentyFour = true
let seconds = true

const show = () => {
  const text = new Date().toLocaleTimeString("en-GB", {
    hour: "2-digit",
    minute: "2-digit",
    second: seconds ? "2-digit" : undefined,
    hour12: !twentyFour,
  })
  item.setLabel(text)
}

item.on("click", ({ button }) => {
  if (button === "left") twentyFour = !twentyFour
  if (button === "right") seconds = !seconds
  show()
})
item.on("error", (error) => console.error(`bar refused: ${error.code}`))

show()
setInterval(show, 1000)
