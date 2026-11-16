// Now playing in the bar, built only on the public SDK (v0.8 dogfood). Shows "Artist - Title" of
// whatever Windows' media controls say is playing (Spotify, browsers, players). Click for
// Previous / Play-Pause / Next. Hidden while nothing plays.
// Run with the bar running: `pnpm now-playing` (build the helper first: `cargo build -p barista-win-helper`).
import * as BA from "@baristabar/sdk"
import { MusicNote01Icon, PauseIcon, PlayIcon } from "@hugeicons/core-free-icons"
import { startHelper } from "./helper.ts"

interface MediaEvent {
  type: "media"
  active: boolean
  playing: boolean
  title: string
  artist: string
  app: string
  canPrevious: boolean
  canNext: boolean
}

const isMediaEvent = (e: unknown): e is MediaEvent =>
  typeof e === "object" && e !== null && (e as { type?: unknown }).type === "media"

/** The bar draws a label up to 128 characters; a long title would also push other items away. */
const MAX_LABEL = 40

function shorten(text: string): string {
  return text.length > MAX_LABEL ? `${text.slice(0, MAX_LABEL - 1)}…` : text
}

/** `Spotify.exe` or `Chrome` from an app id like `Spotify.exe` or `com.squirrel.Spotify`. */
function appName(id: string): string {
  const last = id.split(/[\\/!]/).pop() ?? id
  return last.replace(/\.exe$/i, "")
}

const bar = await BA.connect({ appName: "Now playing" })
bar.on("close", () => process.exit(0))
bar.on("quit", () => process.exit(0))

const item = await BA.StatusItem.create({
  id: "media",
  icon: { icon: MusicNote01Icon, color: "#ffffff", height: 14 },
})
item.on("error", (error) => console.error(`bar refused: ${error.code}`))
// Nothing playing yet: stay out of the bar until the helper says there is something.
item.setVisible(false)

const helper = startHelper(
  "--media",
  (event) => {
    if (!isMediaEvent(event)) return
    item.setVisible(event.active && event.title !== "")
    const text = [event.artist, event.title].filter(Boolean).join(" - ")
    item.setLabel(shorten(text))
    item.setIcon({ icon: event.playing ? PauseIcon : PlayIcon, color: "#ffffff", height: 12 })
    item.setTooltip(text ? `${text} (${appName(event.app)})` : "Now playing")
    item.updateMenuItem("title", { label: shorten(text) || "Nothing playing" })
    item.updateMenuItem("previous", { enabled: event.canPrevious })
    item.updateMenuItem("play-pause", { label: event.playing ? "Pause" : "Play" })
    item.updateMenuItem("next", { enabled: event.canNext })
  },
  () => process.exit(1),
)

// The menu is set right away: the helper's first event comes later (this runs in one go).
const MENU = [
  { id: "title", label: "Nothing playing", enabled: false },
  { type: "separator" as const },
  { id: "previous", label: "Previous", click: () => helper.send({ cmd: "previous" }) },
  { id: "play-pause", label: "Play", click: () => helper.send({ cmd: "playPause" }) },
  { id: "next", label: "Next", click: () => helper.send({ cmd: "next" }) },
]

item.setMenu(MENU)

process.once("SIGINT", () => process.exit(0))
process.on("exit", () => {
  helper.close()
  item.remove()
})
