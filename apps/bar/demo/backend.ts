// A pretend Rust side for the website demo (ADR 0037). `mockIPC` answers the real, generated
// `commands`, so the shapes below are type-checked against bindings.ts: a changed command or event
// fails `pnpm typecheck` here instead of breaking the landing page quietly.
//
// It keeps real state. Flip a toggle, drag a slider, clear a notification, press play: the matching
// event goes out, exactly like Rust would send it, and the real components redraw.
import type { InvokeArgs } from "@tauri-apps/api/core"
import { emit } from "@tauri-apps/api/event"
import type {
  AirplaneMode,
  AppVolumes_Serialize,
  Battery_Serialize,
  BarAppearance,
  Bluetooth,
  Brightness,
  ClockSettings,
  Hotspot,
  Media_Serialize,
  MenuEntry_Serialize,
  MenuOpen_Serialize,
  NightLight,
  Notifications_Serialize,
  OutputDevices,
  PanelOpen,
  Settings_Serialize,
  StatusItemView_Serialize,
  ThemeMode,
  User_Serialize,
  Volume,
  Wifi_Serialize,
} from "@/shared/bindings"
import defaultSettings from "./settings.json"

const send = (event: string, payload: unknown) => void emit(event, payload)

// ---------------------------------------------------------------- what the bar shows

// `?scene=sdk` is the SDK section of the landing page: the real bar with nothing on it but one app's item.
const SDK_SCENE = new URLSearchParams(location.search).get("scene") === "sdk"

const baseSettings = defaultSettings as unknown as Settings_Serialize
const settings: Settings_Serialize = SDK_SCENE
  ? { ...baseSettings, menuBar: { ...baseSettings.menuBar, items: baseSettings.menuBar.items.map((i) => ({ ...i, visible: false })) } }
  : baseSettings

const item = (
  id: string,
  appName: string,
  label: string | null,
  extra: Partial<StatusItemView_Serialize> = {},
): StatusItemView_Serialize => ({
  id,
  appName,
  icon: null,
  label,
  tooltip: null,
  source: "app",
  pinned: true,
  placement: "bar",
  color: null,
  script: null,
  ...extra,
})

const entry = (id: string | null, label: string, extra: Partial<MenuEntry_Serialize> = {}): MenuEntry_Serialize => ({
  id,
  kind: id ? "normal" : "separator",
  label,
  checked: false,
  enabled: true,
  accelerator: null,
  role: null,
  icon: null,
  info: false,
  color: null,
  submenu: [],
  ...extra,
})

// ---------------------------------------------------------------- the focus timer (an SDK app)

const TIMER = "demo/timer"
const timer = { minutes: 25, left: 25 * 60, running: false, sounds: true, ticker: 0 as unknown as ReturnType<typeof setInterval> }
const clockText = (s: number) => `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`

// The counter from the SDK example on the landing page: click it and the label goes up.
const COUNTER = "demo/counter"
let count = 0

const items: StatusItemView_Serialize[] = SDK_SCENE
  ? [item(COUNTER, "Counter", "0", { tooltip: "Click me" })]
  : [item(TIMER, "Focus", clockText(timer.left), { tooltip: "Focus timer" })]

function timerMenu(): MenuEntry_Serialize[] {
  return [
    entry("start", timer.running ? "Pause" : "Start", { accelerator: "Ctrl+S" }),
    entry("reset", "Reset", { enabled: timer.left !== timer.minutes * 60 }),
    entry(null, ""),
    entry("p25", "25 minutes", { kind: "radio", checked: timer.minutes === 25 }),
    entry("p50", "50 minutes", { kind: "radio", checked: timer.minutes === 50 }),
    entry(null, ""),
    entry("sounds", "Sounds", { kind: "checkbox", checked: timer.sounds }),
  ]
}

function showTimer() {
  items[0] = { ...items[0]!, label: clockText(timer.left) }
  send("items-changed", items)
  send("menu-changed", { itemId: TIMER, menu: timerMenu() })
}

function tick() {
  timer.left -= 1
  if (timer.left <= 0) {
    clearInterval(timer.ticker)
    timer.running = false
    timer.left = timer.minutes * 60
  }
  showTimer()
}

function pickTimerEntry(id: string): "close" | "stay" {
  if (id === "start") {
    timer.running = !timer.running
    clearInterval(timer.ticker)
    if (timer.running) timer.ticker = setInterval(tick, 1000)
    showTimer()
    return "close"
  }
  if (id === "reset") {
    clearInterval(timer.ticker)
    timer.running = false
    timer.left = timer.minutes * 60
    showTimer()
    return "close"
  }
  if (id === "p25" || id === "p50") {
    timer.minutes = id === "p25" ? 25 : 50
    if (!timer.running) timer.left = timer.minutes * 60
    showTimer()
    return "stay"
  }
  if (id === "sounds") {
    timer.sounds = !timer.sounds
    showTimer()
    return "stay"
  }
  return "close"
}

// ---------------------------------------------------------------- system state

const state = {
  airplane: false,
  bluetooth: true,
  hotspot: false,
  nightLight: false,
  dark: true,
  saver: false,
  brightness: 0.7,
  volume: 0.6,
  muted: false,
  wifi: true,
  device: "speakers",
  track: 0,
  playing: true,
  seen: false,
  appVolumes: [
    { id: "music", name: "Barista Radio", level: 0.8, muted: false, playing: true },
    { id: "browser", name: "Browser", level: 0.5, muted: false, playing: false },
  ],
}

const TRACKS = [
  { title: "Morning Light", artist: "Kalk Bay Tides" },
  { title: "Cape Doctor", artist: "Table Mountain" },
  { title: "Lion's Head at Dusk", artist: "Barista Radio" },
]

const snap = {
  battery: (): Battery_Serialize => ({
    present: true,
    percent: 82,
    charging: false,
    pluggedIn: false,
    secondsLeft: 4 * 3600,
    saver: state.saver,
    saverControllable: true,
  }),
  volume: (): Volume => ({ available: true, level: state.volume, muted: state.muted, deviceName: "Speakers" }),
  airplane: (): AirplaneMode => ({ supported: true, on: state.airplane }),
  bluetooth: (): Bluetooth => ({
    available: true,
    enabled: state.bluetooth && !state.airplane,
    accessDenied: false,
    devices: [
      { id: "buds", name: "Studio Buds", connected: true },
      { id: "kb", name: "Keyboard", connected: false },
    ],
  }),
  hotspot: (): Hotspot => ({
    supported: true,
    state: state.hotspot ? "on" : "off",
    clients: state.hotspot ? 1 : 0,
    maxClients: 8,
  }),
  nightLight: (): NightLight => ({ supported: true, on: state.nightLight }),
  theme: (): ThemeMode => ({ dark: state.dark }),
  brightness: (): Brightness => ({
    displays: [{ id: "internal", name: "Built-in display", internal: true, supported: true, level: state.brightness }],
  }),
  wifi: (): Wifi_Serialize => ({
    available: true,
    enabled: state.wifi && !state.airplane,
    accessDenied: false,
    locationAllowed: true,
    current: state.wifi && !state.airplane ? "Home" : null,
    networks:
      state.wifi && !state.airplane
        ? [
            { ssid: "Home", signal: 90, secured: true, known: true, connected: true },
            { ssid: "Kalk Bay Cafe", signal: 60, secured: false, known: false, connected: false },
          ]
        : [],
  }),
  media: (): Media_Serialize => ({
    available: true,
    source: "barista-radio",
    appName: "Barista Radio",
    title: TRACKS[state.track]!.title,
    artist: TRACKS[state.track]!.artist,
    playing: state.playing,
    canNext: true,
    canPrevious: true,
    cover: null,
  }),
  devices: (): OutputDevices => ({
    canSwitch: true,
    devices: [
      { id: "speakers", name: "Speakers", default: state.device === "speakers" },
      { id: "headphones", name: "Headphones", default: state.device === "headphones" },
    ],
  }),
  appVolumes: (): AppVolumes_Serialize => ({ apps: state.appVolumes.map((a) => ({ ...a, icon: null })) }),
  user: (): User_Serialize => ({ available: true, name: "Liam", email: null, avatar: null }),
}

const clock: ClockSettings = {
  hour24: false,
  firstWeekday: 1,
  timeZone: "",
  utcOffsetMinutes: -new Date().getTimezoneOffset(),
}
// The bar uses its dark look on the light wallpaper (docs/design/Abstract-12.jpg): light text on a dark strip.
const appearance: BarAppearance = { tone: "dark" }

const now = Math.floor(Date.now() / 1000)
const notifications: Notifications_Serialize = {
  access: "allowed",
  unread: true,
  groups: [
    {
      appId: "mail",
      appName: "Mail",
      items: [
        { id: 1, title: "Welcome to Barista", lines: ["Your tray icons live in the bar now."], attribution: null, arrivedAt: now - 120, hasImage: false },
        { id: 2, title: "Lunch at Kalk Bay?", lines: ["Fish and chips, 1pm. Bring the laptop."], attribution: null, arrivedAt: now - 900, hasImage: false },
      ],
    },
    {
      appId: "calendar",
      appName: "Calendar",
      items: [
        { id: 3, title: "Design review in 10 minutes", lines: ["Room 2, or the usual link."], attribution: null, arrivedAt: now - 300, hasImage: false },
      ],
    },
  ],
}

// ---------------------------------------------------------------- panels and menus

let panel: PanelOpen | null = null
const rect = { x: 0, y: 0, width: 0, height: 0, left: 0, top: 0, right: 0, bottom: 0 }

/** The size each pretend window asked for, so the stage can give it a box (see stage.tsx). */
export type Size = { width: number; height: number } | null
const sizes: Record<"panel" | "menu", Size> = { panel: null, menu: null }
const listeners = new Set<() => void>()
export const subscribeSizes = (cb: () => void) => (listeners.add(cb), () => void listeners.delete(cb))
export const getSize = (which: "panel" | "menu") => sizes[which]
function setSize(which: "panel" | "menu", width: unknown, height: unknown) {
  const next = typeof width === "number" && typeof height === "number" ? { width, height } : null
  if (sizes[which]?.width === next?.width && sizes[which]?.height === next?.height) return
  sizes[which] = next
  listeners.forEach((l) => l())
}

function openPanel(name: string, anchor: string) {
  if (name === "notifications") {
    state.seen = true
    notifications.unread = false
  }
  panel = { name, anchor, rect, scale: 1, monitorId: "demo", acrylic: true, chrome: "glass" }
  send("panel-changed", { open: panel })
}

function closePanel() {
  setSize("panel", null, null)
  panel = null
  send("panel-changed", { open: null })
}

function closeMenu() {
  setSize("menu", null, null)
  send("menu-close", { itemId: TIMER })
}

const clamp01 = (n: unknown) => Math.min(1, Math.max(0, typeof n === "number" ? n : 0))

/** The one answer function `mockIPC` calls for every command. */
export function handle(cmd: string, payload?: InvokeArgs): unknown {
  const args = payload as Record<string, unknown> | undefined
  switch (cmd) {
    // -- reads
    case "get_items":
      return items
    case "get_collapsed":
      return false
    case "get_settings":
      return settings
    case "get_battery":
      return snap.battery()
    case "get_volume":
      return snap.volume()
    case "get_clock_settings":
      return clock
    case "get_bar_appearance":
      return appearance
    case "get_wifi":
      return snap.wifi()
    case "get_bluetooth":
      return snap.bluetooth()
    case "get_airplane_mode":
      return snap.airplane()
    case "get_hotspot":
      return snap.hotspot()
    case "get_night_light":
      return snap.nightLight()
    case "get_theme_mode":
      return snap.theme()
    case "get_brightness":
      return snap.brightness()
    case "get_media":
      return snap.media()
    case "get_output_devices":
      return snap.devices()
    case "get_app_volumes":
      return snap.appVolumes()
    case "get_user":
      return snap.user()
    case "get_notifications":
      return notifications
    case "get_tray_style":
      return "panel"
    case "get_control_center_mode":
    case "get_notification_mode":
      return "barista"
    case "get_window_radius":
      return { radius: null, customized: false }
    case "get_open_panel":
      return panel
    case "get_scripts":
    case "get_script_trust_requests":
      return []
    case "get_open_menu":
      return null

    // -- panels and menus
    case "open_panel":
      openPanel(String(args?.name), String(args?.anchor))
      return null
    case "close_panel":
      closePanel()
      return null
    case "panel_resized":
      setSize("panel", args?.width, args?.height)
      return null
    case "menu_resized":
      setSize("menu", args?.width, args?.height)
      return null
    case "close_menu":
      closeMenu()
      return null
    case "item_clicked": {
      if (String(args?.id) === COUNTER) {
        count += 1
        items[0] = { ...items[0]!, label: String(count) }
        send("items-changed", items)
      }
      if (String(args?.id) === TIMER) {
        const open: MenuOpen_Serialize = {
          itemId: TIMER,
          menu: timerMenu(),
          anchor: { item: rect, monitorId: "demo", scale: 1, acrylic: true },
        }
        send("menu-open", open)
      }
      return null
    }
    case "menu_item_clicked":
      if (pickTimerEntry(String(args?.menuItemId)) === "close") closeMenu()
      return null

    // -- quick toggles
    case "set_airplane_mode":
      state.airplane = Boolean(args?.on)
      send("airplane-mode-changed", snap.airplane())
      send("wifi-changed", snap.wifi())
      send("bluetooth-changed", snap.bluetooth())
      return null
    case "set_wifi_enabled":
      state.wifi = Boolean(args?.enabled)
      send("wifi-changed", snap.wifi())
      return null
    case "set_bluetooth_enabled":
      state.bluetooth = Boolean(args?.enabled)
      send("bluetooth-changed", snap.bluetooth())
      return null
    case "set_hotspot_enabled":
      state.hotspot = Boolean(args?.enabled)
      send("hotspot-changed", snap.hotspot())
      return null
    case "set_night_light":
      state.nightLight = Boolean(args?.on)
      send("night-light-changed", snap.nightLight())
      return null
    case "set_dark_mode":
      state.dark = Boolean(args?.dark)
      send("theme-mode-changed", snap.theme())
      return null
    case "set_battery_saver":
      state.saver = Boolean(args?.on)
      send("battery-changed", snap.battery())
      return null

    // -- sliders
    case "set_volume":
      state.volume = clamp01(args?.level)
      state.muted = false
      send("volume-changed", snap.volume())
      return null
    case "set_muted":
      state.muted = Boolean(args?.muted)
      send("volume-changed", snap.volume())
      return null
    case "set_brightness":
      state.brightness = clamp01(args?.level)
      send("brightness-changed", snap.brightness())
      return null
    case "set_app_volume": {
      const app = state.appVolumes.find((a) => a.id === args?.id)
      if (app) app.level = clamp01(args?.level)
      send("app-volumes-changed", snap.appVolumes())
      return null
    }
    case "set_app_muted": {
      const app = state.appVolumes.find((a) => a.id === args?.id)
      if (app) app.muted = Boolean(args?.muted)
      send("app-volumes-changed", snap.appVolumes())
      return null
    }
    case "set_output_device":
      state.device = String(args?.id)
      send("output-devices-changed", snap.devices())
      return null

    // -- media
    case "media_play_pause":
      state.playing = !state.playing
      send("media-changed", snap.media())
      return null
    case "media_next":
      state.track = (state.track + 1) % TRACKS.length
      send("media-changed", snap.media())
      return null
    case "media_previous":
      state.track = (state.track + TRACKS.length - 1) % TRACKS.length
      send("media-changed", snap.media())
      return null

    // -- notifications
    case "dismiss_notification": {
      for (const group of notifications.groups) group.items = group.items.filter((n) => n.id !== args?.id)
      notifications.groups = notifications.groups.filter((g) => g.items.length > 0)
      send("notifications-changed", notifications)
      return null
    }
    case "clear_notifications":
      notifications.groups = []
      send("notifications-changed", notifications)
      return null
    case "mark_notifications_seen":
      notifications.unread = false
      send("notifications-changed", notifications)
      return null

    default:
      // Everything the demo does not pretend about: nothing happens, and a note for whoever builds on it.
      console.debug("demo: no answer for", cmd)
      return null
  }
}
