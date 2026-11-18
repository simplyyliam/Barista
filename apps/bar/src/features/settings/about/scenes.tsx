import { useEffect, useState, type CSSProperties, type ReactNode } from "react"
import { motion, useReducedMotion, type Transition } from "motion/react"
import {
  Activity01Icon,
  AirplaneModeIcon,
  BatteryEcoChargingIcon,
  BluetoothIcon,
  HotspotIcon,
  Icon,
  Mic01Icon,
  MusicNote01Icon,
  Moon02Icon,
  Wifi01Icon,
} from "@barista/ui/icons"
import { Switch } from "@barista/ui/components/ui/switch"
import { cn } from "cn"
import type { BuiltInId } from "@/features/menubar/built-in"
import type { BarItemPref_Serialize } from "@/shared/bindings"
import { Glyph, pillClass } from "../bar-editor"
import { SLIDE, WAVE_HIDDEN, WAVE_SHOWN, staggerDelay, waveEnter } from "@/shared/motion"
import { useSettings } from "@/shared/stores/settings"
import type { SceneId } from "./whats-new"

/**
 * Mock-ups for the About banner, drawn with the bar's real pieces (pills, icons, clock, control
 * center tiles) so they look like Barista. Each plays once, about two seconds, and rests on its last
 * frame: the window idles at zero running animations (ADR 0023, nothing loops).
 */

/** Reduced motion: every scene jumps straight to its last frame. */
function useQuick() {
  const reduce = useReducedMotion()
  return (t: Transition): Transition => (reduce ? { duration: 0 } : t)
}

/** True after `ms`, once. Drives the one change a scene shows. */
function useAfter(ms: number) {
  const [done, setDone] = useState(false)
  useEffect(() => {
    const id = setTimeout(() => setDone(true), ms)
    return () => clearTimeout(id)
  }, [ms])
  return done
}

type Pref = BarItemPref_Serialize

/**
 * The bar of the menu bar preview, without the dragging: the same pieces (`Glyph`, `pillClass`), the
 * same layout and sizes, and the saved look. `lit` is the item shown picked up; `plate` is the solid
 * background; `extra` are apps' items that sit before the right-hand built-ins.
 */
function StaticBar({
  right = ["battery", "wifi", "volume", "search", "control-center", "clock"],
  lit,
  plate,
  extra,
}: {
  right?: string[]
  lit?: string
  plate?: boolean
  extra?: ReactNode
}) {
  const t = useQuick()
  const settings = useSettings()
  if (!settings) return null
  const prefs = settings.menuBar.items
  const prefOf = (id: string): Pref => prefs.find((p) => p.id === id) ?? ({ id, visible: true } as Pref)
  const item = (id: string) => {
    const pref = prefOf(id)
    return (
      <motion.span key={id} layout transition={t(SLIDE)} className={cn(pillClass(pref), lit === id && "bg-bar-hover")}>
        <Glyph id={id as BuiltInId} pref={pref} settings={settings} />
      </motion.span>
    )
  }
  return (
    <div
      data-bar-root
      style={{ "--bar-h": `${settings.menuBar.height}px` } as CSSProperties}
      // A plain string, not `cn()`: tailwind-merge takes `text-bar` (size) and `text-bar-fg` (colour)
      // for the same thing and drops the size.
      className={`flex w-[calc(100%-24px)] items-center justify-between rounded-lg font-bar text-bar font-bar-item text-bar-fg transition-colors duration-500 ${plate ? "bg-[#f4f4f4]" : ""}`}
    >
      <div className="flex h-[var(--bar-h)] items-center">
        {["brand", "app-name"].map(item)}
      </div>
      <div className="flex h-[var(--bar-h)] items-center gap-[2px]">
        {extra}
        {right.map(item)}
      </div>
    </div>
  )
}

/** Wi-Fi and battery trade places: the others glide aside. */
function BarScene() {
  const moved = useAfter(700)
  return (
    <StaticBar
      lit="wifi"
      right={moved ? ["wifi", "battery", "volume", "search", "control-center", "clock"] : ["battery", "wifi", "volume", "search", "control-center", "clock"]}
    />
  )
}

/** Apps' items arrive in the bar one after another, like the real wave. */
function ItemsScene() {
  const t = useQuick()
  const icons = [Mic01Icon, MusicNote01Icon, Activity01Icon]
  return (
    <StaticBar
      extra={icons.map((icon, index) => (
        <motion.span
          key={index}
          initial={WAVE_HIDDEN}
          animate={WAVE_SHOWN}
          transition={t(waveEnter(0.3 + staggerDelay(index)))}
          className={pillClass({ id: "app" } as Pref)}
        >
          <Icon icon={icon} size={16} strokeWidth={1.6} />
        </motion.span>
      ))}
    />
  )
}

/** A setting flips and the bar answers at once. */
function SettingsScene() {
  const on = useAfter(700)
  return (
    <div className="flex w-full flex-col items-center gap-4">
      <StaticBar plate={on} />
      <div className="flex h-10 w-64 items-center justify-between rounded-xl bg-black/85 px-3.5 text-[13px] text-foreground backdrop-blur-sm">
        Solid background
        <Switch checked={on} aria-label="Solid background" className="pointer-events-none" tabIndex={-1} />
      </div>
    </div>
  )
}

const TILES = {
  wifi: { icon: Wifi01Icon, label: "Wi-Fi", sub: "Home", wide: true, active: true },
  airplane: { icon: AirplaneModeIcon },
  night: { icon: Moon02Icon },
  bluetooth: { icon: BluetoothIcon },
  hotspot: { icon: HotspotIcon },
  saver: { icon: BatteryEcoChargingIcon, label: "Battery Saver", wide: true },
} as const

/** Two tiles trade places in a control center card (the real tile look). */
function ControlScene() {
  const t = useQuick()
  const swapped = useAfter(700)
  const order = swapped
    ? (["wifi", "night", "airplane", "bluetooth", "hotspot", "saver"] as const)
    : (["wifi", "airplane", "night", "bluetooth", "hotspot", "saver"] as const)
  return (
    <div className="grid w-[330px] grid-cols-4 gap-cc-gap rounded-panel bg-cc-card p-cc-pad text-cc-fg">
      {order.map((id) => {
        const tile = TILES[id] as { icon: typeof Wifi01Icon; label?: string; sub?: string; wide?: boolean; active?: boolean }
        return (
          <motion.div
            key={id}
            layout
            transition={t(SLIDE)}
            className={cn(
              "flex h-cc-tile items-center rounded-panel-child select-none",
              tile.active ? "bg-cc-active" : "bg-cc-tile",
              tile.wide ? "col-span-2 justify-start gap-2.5 px-cc-label-pad" : "justify-center",
            )}
          >
            <Icon icon={tile.icon} size={tile.wide ? 20 : 24} className="shrink-0" />
            {tile.wide && (
              <span className="flex min-w-0 flex-col leading-tight">
                <span className="truncate text-[11px] font-medium">{tile.label}</span>
                {tile.sub && <span className="truncate text-[10px] text-cc-fg-muted">{tile.sub}</span>}
              </span>
            )}
          </motion.div>
        )
      })}
    </div>
  )
}

export function Scene({ id }: { id: SceneId }) {
  switch (id) {
    case "bar":
      return <BarScene />
    case "control":
      return <ControlScene />
    case "items":
      return <ItemsScene />
    case "settings":
      return <SettingsScene />
  }
}
