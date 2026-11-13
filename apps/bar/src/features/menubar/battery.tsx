import { useBattery } from "@/shared/hooks/use-system-state"
import { BatteryIcon } from "./icons"
import { MenubarItem } from "./menubar-item"

type BatteryViewProps = {
  percent: number | null
  charging: boolean
  /** The percentage as a 13px label left of the icon (off by default, like macOS). */
  showLabel?: boolean
  /** The percentage inside the shell while not charging. */
  showPercentInside?: boolean
  open?: boolean
}

/** The battery pill from plain values (the debug window feeds it made-up ones). */
export function BatteryView({ percent, charging, showLabel = false, showPercentInside = false, open }: BatteryViewProps) {
  return (
    <MenubarItem open={open} className="cursor-default gap-1.5">
      {showLabel && percent !== null && <span className="tabular-nums">{Math.round(percent)}%</span>}
      <BatteryIcon percent={percent} charging={charging} showPercent={showPercentInside} />
    </MenubarItem>
  )
}

/** Battery level and charging from the battery service. Not shown on a desktop PC. */
export function Battery(props: Pick<BatteryViewProps, "showLabel" | "showPercentInside">) {
  const battery = useBattery()
  if (!battery?.present) return null
  return <BatteryView percent={battery.percent ?? null} charging={battery.charging} {...props} />
}
