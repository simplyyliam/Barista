// System monitor in the bar (v0.8 dogfood, built only on the public SDK): "CPU 12%" in the bar,
// click for two small graphs (the last 60 seconds of CPU and RAM) in a popover window.
// Not bundled with Barista: it is an Electron app like any other client (ADR 0030).
// Run with the bar running: `pnpm system-monitor`.
import { fileURLToPath } from "node:url"
import { app, BrowserWindow, ipcMain, screen } from "electron"
import * as BA from "@baristabar/sdk"
import { electronPopover } from "@baristabar/sdk/electron"
import { CpuIcon } from "@hugeicons/core-free-icons"
import { cpuPercent, cpuTimes, memory, push, type CpuTimes } from "./sampler.ts"

const cpuHistory: number[] = []
const ramHistory: number[] = []

async function main(): Promise<void> {
  let bar: BA.StatusBar
  try {
    bar = await BA.connect({ appName: "System monitor" })
  } catch (error) {
    console.error((error as Error).message)
    app.exit(1)
    return
  }
  bar.on("close", () => app.exit(0))
  bar.on("quit", () => app.quit())
  await app.whenReady()

  const win = new BrowserWindow({
    show: false,
    frame: false,
    resizable: false,
    skipTaskbar: true,
    alwaysOnTop: true,
    backgroundColor: "#202124",
    webPreferences: {
      preload: fileURLToPath(new URL("./preload.cjs", import.meta.url)),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
    },
  })
  win.webContents.setWindowOpenHandler(() => ({ action: "deny" }))
  win.webContents.on("will-navigate", (event) => event.preventDefault())

  let latest = { cpu: 0, ram: 0, usedGb: 0, totalGb: 0 }
  const snapshot = () => ({ cpuHistory, ramHistory, ...latest })
  // The page asks once when it opens; after that it gets one push a second while the popover is open.
  ipcMain.handle("load", (event) => (event.sender === win.webContents ? snapshot() : null))
  await win.loadFile(fileURLToPath(new URL("./monitor.html", import.meta.url)))

  const item = await BA.StatusItem.create({
    id: "monitor",
    icon: { icon: CpuIcon, color: "#ffffff", height: 14 },
    label: "CPU --%",
    tooltip: "System monitor",
  })
  item.on("error", (error) => console.error(`bar refused: ${error.code}: ${error.message}`))
  item.setPopover({ window: electronPopover(win, { screen }), width: 300, height: 232 })
  item.setMenu([{ role: "quit" }])

  let open = false
  item.on("open", () => (open = true))
  item.on("close", () => (open = false))

  let before: CpuTimes | undefined
  let shown = ""
  const sample = () => {
    const now = cpuTimes()
    const cpu = cpuPercent(before, now)
    before = now
    const mem = memory()
    latest = { cpu, ram: mem.percent, usedGb: mem.usedGb, totalGb: mem.totalGb }
    push(cpuHistory, cpu)
    push(ramHistory, mem.percent)
    // Only touch the bar when the text really changes.
    const label = `CPU ${cpu}%`
    if (label !== shown) {
      shown = label
      item.setLabel(label)
    }
    item.setTooltip(`CPU ${cpu}%, memory ${mem.percent}% (${mem.usedGb} of ${mem.totalGb} GB)`)
    if (open && !win.isDestroyed()) win.webContents.send("sample", snapshot())
  }
  sample()
  const timer = setInterval(sample, 1000)

  app.on("window-all-closed", () => {})
  process.once("SIGINT", () => app.quit())
  app.on("before-quit", () => {
    clearInterval(timer)
    item.remove()
    bar.close()
  })
}

void main()
