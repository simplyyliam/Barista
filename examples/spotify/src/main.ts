// A Spotify mini player in the menu bar, built with an Electron popover. Left click shows the
// player window under the icon; right click shows a menu. Electron is a dev dependency of this
// example only: the SDK never imports it.
//
// Where things live:
//   main process (this file): the bar connection, the Spotify tokens, every network call.
//   player page (player.html): draws the state it is sent. It never sees a token; it can only ask
//   for "play" / "pause" / "next" / "previous" through the preload bridge, and we check it here.
//
// Run with the bar running: set SPOTIFY_CLIENT_ID (see README) and `pnpm --filter
// @barista/spotify-demo start`, or `start:demo` for a pretend player.
import { fileURLToPath } from "node:url"
import { app, BrowserWindow, ipcMain, screen } from "electron"
import * as BA from "@barista/sdk"
import { electronPopover } from "@barista/sdk/electron"
import { DemoPlayer } from "./demo.ts"
import { openExternalUrl } from "./open-url.ts"
import { SpotifyClient, type SpotifyTrack } from "./spotify.ts"

type Action = "play" | "pause" | "next" | "previous"
const ACTIONS = new Set<unknown>(["play", "pause", "next", "previous"])

interface Player {
  currentlyPlaying(): Promise<SpotifyTrack | null>
  playback(action: Action): Promise<void>
}

/** What the page draws. */
interface PlayerState {
  status: "setup" | "idle" | "playing" | "paused" | "error"
  title: string
  subtitle: string
  playing: boolean
}

const MUSIC_ICON =
  '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#1db954" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M10 15V8l7-1v7"/><circle cx="8" cy="16" r="2"/><circle cx="15" cy="14" r="2"/></svg>'

const demo = process.argv.includes("--demo")
const clientId = process.env.SPOTIFY_CLIENT_ID?.trim()

let player: Player | undefined = demo ? new DemoPlayer() : undefined
let track: SpotifyTrack | null = null
let state: PlayerState = {
  status: "setup",
  title: "Spotify",
  subtitle: clientId || demo ? "Connect to start" : "Set SPOTIFY_CLIENT_ID to sign in",
  playing: false,
}

async function main(): Promise<void> {
  let bar: BA.StatusBar
  try {
    bar = await BA.connect({ appName: "Spotify" })
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
    backgroundColor: "#181818",
    webPreferences: {
      preload: fileURLToPath(new URL("./preload.cjs", import.meta.url)),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
    },
  })
  // The page is ours and static: it never opens windows or navigates anywhere.
  win.webContents.setWindowOpenHandler(() => ({ action: "deny" }))
  win.webContents.on("will-navigate", (event) => event.preventDefault())
  await win.loadFile(fileURLToPath(new URL("./player.html", import.meta.url)))

  const item = await BA.StatusItem.create({
    id: "spotify",
    icon: Buffer.from(MUSIC_ICON),
    tooltip: "Spotify",
  })
  item.setPopover({ window: electronPopover(win, { screen }), width: 320, height: 380 })
  item.on("error", (error) => console.error(`bar refused: ${error.code}: ${error.message}`))

  const push = () => win.webContents.send("state", state)

  function updateMenu(): void {
    const url = track?.url
    item.setMenu([
      {
        id: "open-track",
        label: "Open track in Spotify",
        enabled: Boolean(url),
        click: () => (url ? void openExternalUrl(url).catch(report) : undefined),
      },
      { type: "separator" },
      { role: "quit" },
    ])
  }

  function report(error: unknown): void {
    const message = error instanceof Error ? error.message : String(error)
    console.error(`[spotify] ${message}`)
    state = { status: "error", title: "Something went wrong", subtitle: message.slice(0, 120), playing: false }
    push()
  }

  async function refresh(): Promise<void> {
    if (!player) return
    try {
      track = await player.currentlyPlaying()
    } catch (error) {
      return report(error)
    }
    state = track
      ? {
          status: track.isPlaying ? "playing" : "paused",
          title: track.name,
          subtitle: track.artists.join(", "),
          playing: track.isPlaying,
        }
      : { status: "idle", title: "Nothing playing", subtitle: "Start Spotify on any device", playing: false }
    item.setTooltip((track ? `${track.name} — ${track.artists.join(", ")}` : "Spotify").slice(0, 512))
    updateMenu()
    push()
  }

  // Only our own window may ask for anything, and only for the four known actions.
  ipcMain.handle("playback", async (event, action: unknown) => {
    if (event.sender !== win.webContents || !ACTIONS.has(action) || !player) return
    try {
      await player.playback(action as Action)
      // Spotify's state lags a moment behind the command.
      setTimeout(() => void refresh(), 400)
    } catch (error) {
      report(error)
    }
  })
  ipcMain.handle("connect", async (event) => {
    if (event.sender !== win.webContents || player) return
    if (!clientId) return report(new Error("Set SPOTIFY_CLIENT_ID first (see the README)"))
    state = { ...state, subtitle: "Finish signing in in your browser…" }
    push()
    try {
      player = await SpotifyClient.authorize(clientId)
      await refresh()
    } catch (error) {
      report(error)
    }
  })
  ipcMain.on("ready", (event) => {
    if (event.sender === win.webContents) push()
  })

  // The popover is about to show: get fresh state so the first frame is right.
  item.on("open", () => void refresh())
  updateMenu()
  void refresh()
  app.on("window-all-closed", () => {})
  process.once("SIGINT", () => app.quit())
  app.on("before-quit", () => {
    item.remove()
    bar.close()
  })
}

void main()
