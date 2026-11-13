import { useBattery } from "@/shared/hooks/use-system-state"
import { BatteryIcon } from "./icons"
import { MenubarItem } from "./menubar-item"

type BatteryProps = {
  /** The percentage as a 13px label left of the icon (off by default, like macOS). */
  showLabel?: boolean
  /** The percentage inside the shell while not charging. */
  showPercentInside?: boolean
}

/** Battery level and charging. Not shown on a desktop PC. */
export function Battery({ showLabel = false, showPercentInside = false }: BatteryProps) {
  const battery = useBattery()
  if (!battery?.present) return null
  const percent = battery.percent ?? null

  return (
    <MenubarItem className="cursor-default gap-1.5">
      {showLabel && percent !== null && <span className="tabular-nums">{Math.round(percent)}%</span>}
      <BatteryIcon percent={percent} charging={battery.charging} showPercent={showPercentInside} />
    </MenubarItem>
  )
}
