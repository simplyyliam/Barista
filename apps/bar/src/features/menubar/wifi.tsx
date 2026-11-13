import { type HTMLAttributes } from "react"
import { cn } from "cn"
import { commands } from "@/shared/bindings"
import { useControlCenterMode, useWifi } from "@/shared/hooks/use-system-state"
import { useOpenPanel } from "@/features/panel"
import { WIFI_PANEL_HEIGHT, WIFI_PANEL_WIDTH } from "@/features/panel/wifi-panel"
import { WifiIcon } from "./icons"
import { useItemBounds } from "./hooks/use-item-bounds"
import { wifiStrength } from "./lib/levels"
import { MenubarItem } from "./menubar-item"

const ITEM_ID = "system/wifi"

type WifiViewProps = HTMLAttributes<HTMLDivElement> & {
  strength: 0 | 1 | 2 | 3
  off?: boolean
  open?: boolean
}

/** The Wi-Fi pill from plain values (the debug window feeds it made-up ones). */
export function WifiView({ strength, off = false, open, className, ...props }: WifiViewProps) {
  return (
    <MenubarItem open={open} className={cn("cursor-default", className)} aria-label="Wi-Fi" {...props}>
      <WifiIcon off={off} strength={strength} />
    </MenubarItem>
  )
}

/**
 * Wi-Fi: arcs follow the signal of the network we are on. Click opens the Wi-Fi panel under the
 * icon (the control center's Wi-Fi view), or Windows' own Quick Settings when the control center
 * is set to "windows". Not shown without a Wi-Fi adapter.
 */
export function Wifi() {
  const wifi = useWifi()
  const mode = useControlCenterMode()
  const open = useOpenPanel()
  const available = wifi?.available === true

  // The panel opens under the item, so Rust needs to know where it is drawn. The item shows up
  // once the adapter is known, so the key changes then and the position is sent again.
  useItemBounds(available ? ITEM_ID : "")

  if (!wifi || !available) return null

  const click = () => {
    if (mode === "windows") {
      void commands.openWindowsControlCenter()
    } else {
      void commands.openPanel("wifi", ITEM_ID, WIFI_PANEL_WIDTH, WIFI_PANEL_HEIGHT, "none")
    }
  }

  const signal = wifi.networks.find((network) => network.connected)?.signal
  return (
    <WifiView
      off={!wifi.enabled}
      strength={wifiStrength(signal)}
      open={open?.anchor === ITEM_ID}
      role="button"
      tabIndex={0}
      aria-expanded={open?.anchor === ITEM_ID}
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
