// A pretend Rust side for the website demo (ADR 0037). `mockIPC` answers the real, generated
// `commands`, so this file is type-checked against bindings.ts: a changed command shape fails
// `pnpm typecheck` here instead of breaking the landing page quietly.
import type { InvokeArgs } from "@tauri-apps/api/core"
import { emit } from "@tauri-apps/api/event"
import type {
  Battery_Serialize,
  BarAppearance,
  ClockSettings,
  MenuEntry_Serialize,
  MenuOpen_Serialize,
  Notifications_Serialize,
  PanelOpen,
  Settings_Serialize,
  StatusItemView_Serialize,
  Volume,
  Wifi_Serialize,
} from "@/shared/bindings"
import defaultSettings from "./settings.json"

const settings = defaultSettings as unknown as Settings_Serialize

const item = (id: string, appName: string, label: string | null, extra: Partial<StatusItemView_Serialize> = {}): StatusItemView_Serialize => ({
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

const items: StatusItemView_Serialize[] = [item("demo/timer", "Focus", "25:00", { tooltip: "Focus timer" })]

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

const menus: Record<string, MenuEntry_Serialize[]> = {
  "demo/timer": [
    entry("start", "Start", { accelerator: "Ctrl+S" }),
    entry("pause", "Pause", { enabled: false }),
    entry(null, ""),
    entry("p25", "25 minutes", { kind: "radio", checked: true }),
    entry("p50", "50 minutes", { kind: "radio" }),
    entry(null, ""),
    entry("sounds", "Sounds", { kind: "checkbox", checked: true }),
  ],
}

const battery: Battery_Serialize = {
  present: true,
  percent: 82,
  charging: false,
  pluggedIn: false,
  secondsLeft: 4 * 3600,
  saver: false,
  saverControllable: true,
}
const volume: Volume = { available: true, level: 0.6, muted: false, deviceName: "Speakers" }
const clock: ClockSettings = { hour24: false, firstWeekday: 1, timeZone: "", utcOffsetMinutes: -new Date().getTimezoneOffset() }
const appearance: BarAppearance = { tone: "dark" }
const wifi: Wifi_Serialize = {
  available: true,
  enabled: true,
  accessDenied: false,
  locationAllowed: true,
  current: "Home",
  networks: [
    { ssid: "Home", signal: 90, secured: true, known: true, connected: true },
    { ssid: "Cafe", signal: 60, secured: false, known: false, connected: false },
  ],
}
const now = Math.floor(Date.now() / 1000)
const notifications: Notifications_Serialize = {
  access: "allowed",
  unread: true,
  groups: [
    {
      appId: "mail",
      appName: "Mail",
      items: [
        { id: 1, title: "Welcome to Barista", lines: ["Tray icons live in the bar now."], attribution: null, arrivedAt: now - 120, hasImage: false },
      ],
    },
  ],
}

let panel: PanelOpen | null = null

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
const rect = { x: 0, y: 0, width: 0, height: 0, left: 0, top: 0, right: 0, bottom: 0 }

function openPanel(name: string, anchor: string) {
  panel = { name, anchor, rect, scale: 1, monitorId: "demo", acrylic: true, chrome: "glass" }
  void emit("panel-changed", { open: panel })
}

/** The one answer function `mockIPC` calls for every command. */
export function handle(cmd: string, payload?: InvokeArgs): unknown {
  const args = payload as Record<string, unknown> | undefined
  switch (cmd) {
    case "get_items":
      return items
    case "get_collapsed":
      return false
    case "get_settings":
      return settings
    case "get_battery":
      return battery
    case "get_volume":
      return volume
    case "get_clock_settings":
      return clock
    case "get_bar_appearance":
      return appearance
    case "get_wifi":
      return wifi
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
    case "open_panel":
      openPanel(String(args?.name), String(args?.anchor))
      return null
    case "panel_resized":
      setSize("panel", args?.width, args?.height)
      return null
    case "menu_resized":
      setSize("menu", args?.width, args?.height)
      return null
    case "close_menu":
      setSize("menu", null, null)
      return null
    case "close_panel":
      setSize("panel", null, null)
      panel = null
      void emit("panel-changed", { open: null })
      return null
    case "item_clicked": {
      const id = String(args?.id)
      const menu = menus[id]
      if (menu) {
        const open: MenuOpen_Serialize = {
          itemId: id,
          menu,
          anchor: { item: rect, monitorId: "demo", scale: 1, acrylic: true },
        }
        void emit("menu-open", open)
      }
      return { status: "ok", data: null }
    }
    default:
      // Everything the demo does not pretend about: nothing happens, and a note for whoever builds on it.
      console.debug("demo: no answer for", cmd)
      return null
  }
}
