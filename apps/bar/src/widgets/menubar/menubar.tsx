import {
  AppName,
  Battery,
  BrandItem,
  Clock,
  ControlCenter,
  MenubarItemGroup,
  Search,
  StatusItems,
  Volume,
  Wifi,
} from "@/features/menubar"
import { useBarHeight } from "@/features/menubar/hooks/use-bar-height"

export default function Menubar() {
  useBarHeight()

  // Fills the bar's height and centres the pills vertically. Sizes: --menubar-* in tokens.css.
  return (
    <div className="grid h-full grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center justify-between px-menubar-edge font-bar text-bar font-bar-item text-bar-fg">
      {/* gap 0: neighbouring hover pills touch, like macOS */}
      <MenubarItemGroup className="justify-self-start">
        <BrandItem />
        <AppName />
      </MenubarItemGroup>
      <MenubarItemGroup className="justify-self-center"></MenubarItemGroup>
      <MenubarItemGroup className="justify-self-end gap-menubar-status-gap">
        <StatusItems />
        <Battery />
        <Wifi />
        <Volume />
        <Search />
        <ControlCenter />
        <Clock />
      </MenubarItemGroup>
    </div>
  )
}
