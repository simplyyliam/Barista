import { useWifi } from "@/shared/hooks/use-system-state"
import { WifiIcon } from "./icons"
import { wifiStrength } from "./lib/levels"
import { MenubarItem } from "./menubar-item"

/** Wi-Fi: arcs follow the signal of the network we are on. Not shown without a Wi-Fi adapter. */
export function Wifi() {
  const wifi = useWifi()
  if (!wifi?.available) return null

  const signal = wifi.networks.find((network) => network.connected)?.signal
  return (
    <MenubarItem className="cursor-default" aria-label="Wi-Fi">
      <WifiIcon off={!wifi.enabled} strength={wifiStrength(signal)} />
    </MenubarItem>
  )
}
