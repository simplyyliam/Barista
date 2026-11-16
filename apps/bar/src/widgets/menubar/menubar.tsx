import {
  AppName,
  Battery,
  BrandItem,
  Clock,
  ControlCenter,
  MenubarItemGroup,
  Search,
  StatusItems,
  TrayItems,
  Volume,
  Wifi,
} from "@/features/menubar"
import { useBarHeight } from "@/features/menubar/hooks/use-bar-height"
import { useItemSpace } from "@/features/menubar/hooks/use-item-space"
import { useResetHover } from "@/features/menubar/hooks/use-reset-hover"
import { useItems } from "@/shared/hooks/use-items"
import { Motion } from "@/shared/motion"

export default function Menubar() {
  useBarHeight()
  useResetHover()
  // Rust splits the items into bar and overflow by the room the bar has.
  useItemSpace(useItems().map((item) => item.id))

  // Fills the bar's height and centres the pills vertically. Sizes: --menubar-* in tokens.css.
  return (
    <Motion>
    <div className="grid h-full grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center justify-between px-menubar-edge font-bar text-bar font-bar-item text-bar-fg">
      {/* gap 0: neighbouring hover pills touch, like macOS */}
      <MenubarItemGroup className="justify-self-start">
        <BrandItem />
        <AppName />
      </MenubarItemGroup>
      <MenubarItemGroup className="justify-self-center" data-bar-center></MenubarItemGroup>
      <MenubarItemGroup className="justify-self-end gap-menubar-status-gap" data-bar-right>
        <TrayItems />
        <StatusItems />
        <Battery />
        <Wifi />
        <Volume />
        <Search />
        <ControlCenter />
        <Clock />
      </MenubarItemGroup>
    </div>
    </Motion>
  )
}
