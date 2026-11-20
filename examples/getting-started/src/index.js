// The five-minute example from the README and the docs. Keep it short: a newcomer pastes it.
// #region item
import * as BA from "@baristabar/sdk"

// Name your app. The bar shows the name in tooltips and in Settings > Connected Apps.
await BA.connect({ appName: "Clock" })

// Add an item to the bar.
const item = await BA.StatusItem.create({ id: "time", label: "12:00", tooltip: "The time" })

// Change it whenever you like.
setInterval(() => item.setLabel(new Date().toLocaleTimeString()), 1000)

// React to clicks.
item.on("click", ({ button }) => console.log("clicked with", button))
// #endregion item
