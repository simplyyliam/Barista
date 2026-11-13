import { useWifi } from "@/shared/hooks/use-system-state"
import { WifiIcon } from "./icons"
import { wifiStrength } from "./lib/levels"
import { MenubarItem } from "./menubar-item"

type WifiViewProps = { strength: 0 | 1 | 2 | 3; off?: boolean; open?: boolean }

/** The Wi-Fi pill from plain values. */
export function WifiView({ strength, off = false, open }: WifiViewProps) {
  return (
    <MenubarItem open={open} className="cursor-default" aria-label="Wi-Fi">
      <WifiIcon off={off} strength={strength} />
    </MenubarItem>
  )
}

/** Wi-Fi: arcs follow the signal of the network we are on. Not shown without a Wi-Fi adapter. */
export function Wifi() {
  const wifi = useWifi()
  if (!wifi?.available) return null
  const signal = wifi.networks.find((network) => network.connected)?.signal
  return <WifiView off={!wifi.enabled} strength={wifiStrength(signal)} />
}
