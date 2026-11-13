import type { ReactNode } from "react"
import { commands } from "@/shared/bindings"
import { useBattery } from "@/shared/hooks/use-system-state"
import { Switch } from "@barista/ui/components/ui/switch"
import { BatteryIcon } from "@/features/menubar/icons"
import { formatDuration } from "./lib/format-duration"

/** Same width as the control center (--spacing-panel-w). */
export const BATTERY_PANEL_WIDTH = 300
/** A first guess; Rust remembers the measured size after the first open. */
export const BATTERY_PANEL_HEIGHT = 232

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex h-10 items-center justify-between rounded-panel-child bg-cc-tile px-cc-label-pad text-[13px] text-cc-fg">
      <span>{label}</span>
      <span className="text-cc-fg-muted">{children}</span>
    </div>
  )
}

/**
 * The `battery` route: level and state, power source, time left and the Battery Saver switch (the
 * same switch the control center tile uses). Opened from the battery item in the bar.
 */
export function BatteryPanel() {
  const battery = useBattery()
  const percent = battery?.percent ?? null
  const charging = battery?.charging === true
  const state = charging ? "Charging" : battery?.pluggedIn ? "Plugged in" : "On battery"

  return (
    <div className="flex w-panel-w flex-col">
      <section className="flex flex-col gap-cc-gap rounded-panel bg-cc-card p-cc-pad text-cc-fg">
        <div className="flex h-cc-header items-center justify-between px-cc-label-pad">
          <div className="flex flex-col leading-tight">
            <span className="text-[24px] font-semibold tabular-nums">{percent === null ? "–" : `${Math.round(percent)}%`}</span>
            <span className="text-[12px] text-cc-fg-muted">{state}</span>
          </div>
          <BatteryIcon percent={percent} charging={charging} pluggedIn={battery?.pluggedIn === true} height={22} />
        </div>
        <Row label="Power source">{battery?.pluggedIn ? "Power adapter" : "Battery"}</Row>
        {!battery?.pluggedIn && battery?.secondsLeft != null && (
          <Row label="Time remaining">{formatDuration(battery.secondsLeft)}</Row>
        )}
        <div className="flex h-10 items-center justify-between rounded-panel-child bg-cc-tile px-cc-label-pad text-[13px]">
          <span>Battery Saver</span>
          <Switch
            aria-label="Battery Saver"
            checked={battery?.saver === true}
            disabled={!battery?.saverControllable}
            onCheckedChange={(on) => void commands.setBatterySaver(on)}
            className="data-checked:bg-cc-fg data-unchecked:bg-cc-fg/30 [&_[data-slot=switch-thumb]]:bg-cc-fg [&_[data-slot=switch-thumb][data-checked]]:bg-cc-active"
          />
        </div>
      </section>
    </div>
  )
}
