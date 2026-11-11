// A quick-note popover for the menu bar (the v0.4 dogfood app): click the pen, type, click away.
// The note is saved to a text file in %APPDATA%/Barista/quick-note. The icon label shows the start of it.
// Run with the bar running: `pnpm --filter @barista/quick-note start`.
import { mkdir, readFile, writeFile } from "node:fs/promises"
import { join } from "node:path"
import { fileURLToPath } from "node:url"
import { app, BrowserWindow, ipcMain, screen } from "electron"
import * as BA from "@barista/sdk"
import { electronPopover } from "@barista/sdk/electron"

const ICON =
  '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#f0f0f0" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"/><path d="M16.4 3.6a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>'
const MAX_NOTE = 100_000

/** The first few characters of the note, for the bar. */
function preview(note: string): string {
  const line = note.trim().split(/\r?\n/, 1)[0] ?? ""
  return line.length > 18 ? `${line.slice(0, 17)}…` : line
}

async function main(): Promise<void> {
  let bar: BA.StatusBar
  try {
    bar = await BA.connect({ appName: "Quick note" })
  } catch (error) {
    console.error((error as Error).message)
    app.exit(1)
    return
  }
  bar.on("close", () => app.exit(0))
  bar.on("quit", () => app.quit())
  await app.whenReady()

  // Not `userData`: started from a file, Electron names the app "Electron" and would use that folder.
  const folder = join(app.getPath("appData"), "Barista", "quick-note")
  const file = join(folder, "note.txt")
  await mkdir(folder, { recursive: true })
  let note = await readFile(file, "utf8").catch(() => "")

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
  // Registered before the page loads: it asks for the note as soon as it starts.
  let item: BA.StatusItem | undefined
  ipcMain.handle("load", (event) => (event.sender === win.webContents ? note : ""))
  ipcMain.handle("save", async (event, text: unknown) => {
    if (event.sender !== win.webContents || typeof text !== "string" || text.length > MAX_NOTE) return
    note = text
    await writeFile(file, note)
    item?.setLabel(preview(note))
  })
  await win.loadFile(fileURLToPath(new URL("./note.html", import.meta.url)))

  const created = await BA.StatusItem.create({
    id: "note",
    icon: Buffer.from(ICON),
    label: preview(note),
    tooltip: "Quick note",
  })
  item = created
  created.setPopover({ window: electronPopover(win, { screen }), width: 360, height: 260 })
  created.setMenu([
    {
      id: "clear",
      label: "Clear note",
      click: async () => {
        note = ""
        await writeFile(file, "")
        created.setLabel("")
        win.webContents.send("show")
      },
    },
    { type: "separator" },
    { role: "quit" },
  ])
  created.on("error", (error) => console.error(`bar refused: ${error.code}: ${error.message}`))

  // Every time it opens: the page reloads the note and puts the cursor in the text box.
  created.on("open", () => win.webContents.send("show"))
  app.on("window-all-closed", () => {})
  process.once("SIGINT", () => app.quit())
  app.on("before-quit", () => {
    created.remove()
    bar.close()
  })
}

void main()
