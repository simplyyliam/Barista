// A tiny client for scripts/lifecycle.mjs: one item, no icon package (the test client needs about
// 10 s to load its icons, which would drown the timings). It uses the built SDK, like any app.
import * as BA from "../packages/sdk/dist/index.js"

const bar = await BA.connect({ appName: process.argv[2] ?? "Focus" })
bar.on("error", () => {})
await BA.StatusItem.create({ id: "timer", label: "25:00" })
// Keep running until the script kills us.
setInterval(() => {}, 60_000)
