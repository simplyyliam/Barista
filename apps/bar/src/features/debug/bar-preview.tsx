import type { CSSProperties } from "react"
import {
  AppName,
  BatteryView,
  BrandItem,
  MenubarItem,
  MenubarItemGroup,
  Search,
  VolumeView,
  WifiView,
} from "@/features/menubar"
import { ControlCenterIcon } from "@/features/menubar/icons"
import { formatClock } from "@/features/menubar/lib/format-clock"
import type { DebugState } from "./debug-state"

/**
 * One strip of the bar: the same pills and icons as the real bar, fed with the debug state.
 * `data-tone` switches the tokens for this strip only, so a light and a dark wallpaper can be
 * shown one above the other.
 */
export function BarPreview({ tone, state, now }: { tone: "light" | "dark"; state: DebugState; now: Date }) {
  const open = state.allOpen
  const { date, time } = formatClock(now, state.hour24)
  return (
    <div
      data-tone={tone}
      style={{ "--menubar-height": `${state.height}px` } as CSSProperties}
      className={`rounded-md ${tone === "light" ? "bg-neutral-300" : "bg-neutral-900"}`}
    >
      <div
        style={{ height: "var(--menubar-height)" }}
        className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center px-menubar-edge font-bar text-bar font-bar-item text-bar-fg"
      >
        <MenubarItemGroup className="justify-self-start">
          <BrandItem open={open} />
          <AppName open={open} />
        </MenubarItemGroup>
        <span />
        <MenubarItemGroup className="justify-self-end gap-menubar-status-gap">
          {state.batteryPresent && (
            <BatteryView
              open={open}
              percent={state.batteryPercent}
              charging={state.charging}
              pluggedIn={state.pluggedIn}
              showLabel={state.showBatteryLabel}
            />
          )}
          <WifiView open={open} strength={state.wifiStrength} off={state.wifiOff} />
          <VolumeView open={open} level={state.volume} muted={state.muted} />
          <Search open={open} />
          <MenubarItem variant="icon" open={open} className="cursor-default" aria-label="Control center">
            <ControlCenterIcon open={open} />
          </MenubarItem>
          <MenubarItem open={open} className="cursor-default gap-1.5 tabular-nums">
            <span>{date}</span>
            <span>{time}</span>
          </MenubarItem>
        </MenubarItemGroup>
      </div>
    </div>
  )
}
