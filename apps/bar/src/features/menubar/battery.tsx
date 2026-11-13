import { type HTMLAttributes } from "react"
import { cn } from "cn"
import { commands } from "@/shared/bindings"
import { useBattery, useControlCenterMode } from "@/shared/hooks/use-system-state"
import { useOpenPanel } from "@/features/panel"
import { BATTERY_PANEL_HEIGHT, BATTERY_PANEL_WIDTH } from "@/features/panel/battery-panel"
import { BatteryIcon } from "./icons"
import { useItemBounds } from "./hooks/use-item-bounds"
import { MenubarItem } from "./menubar-item"

const ITEM_ID = "system/battery"

type BatteryViewProps = HTMLAttributes<HTMLDivElement> & {
  percent: number | null
  charging: boolean
  pluggedIn?: boolean
  /** The percentage as a 13px label left of the icon, like macOS. */
  showLabel?: boolean
  open?: boolean
}

/** The battery pill from plain values (the debug window feeds it made-up ones). */
export function BatteryView({
  percent,
  charging,
  pluggedIn = false,
  showLabel = true,
  open,
  className,
  ...props
}: BatteryViewProps) {
  return (
    <MenubarItem variant={showLabel && percent !== null ? "status" : "icon"} open={open} className={cn("cursor-default gap-1.5 tabular-nums", className)} {...props}>
      {showLabel && percent !== null && <span>{Math.round(percent)}%</span>}
      <BatteryIcon percent={percent} charging={charging} pluggedIn={pluggedIn} />
    </MenubarItem>
  )
}

/**
 * Battery from the battery service: level, charging, plugged in. Click opens the battery panel
 * under the icon (or Windows' Quick Settings when the control center is set to "windows"). Not
 * shown on a desktop PC.
 */
export function Battery({ showLabel }: { showLabel?: boolean }) {
  const battery = useBattery()
  const mode = useControlCenterMode()
  const open = useOpenPanel()
  const present = battery?.present === true

  // The panel opens under the item, so Rust needs to know where it is drawn. The item shows up
  // once the battery is known, so the key changes then and the position is sent again.
  useItemBounds(present ? ITEM_ID : "")

  if (!battery || !present) return null

  const click = () => {
    if (mode === "windows") {
      void commands.openWindowsControlCenter()
    } else {
      void commands.openPanel("battery", ITEM_ID, BATTERY_PANEL_WIDTH, BATTERY_PANEL_HEIGHT, "none")
    }
  }

  const isOpen = open?.anchor === ITEM_ID
  return (
    <BatteryView
      percent={battery.percent ?? null}
      charging={battery.charging}
      pluggedIn={battery.pluggedIn === true}
      showLabel={showLabel}
      open={isOpen}
      role="button"
      tabIndex={0}
      aria-expanded={isOpen}
      data-item-id={ITEM_ID}
      onClick={click}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault()
          click()
        }
      }}
      className="outline-none focus-visible:bg-bar-hover"
    />
  )
}
