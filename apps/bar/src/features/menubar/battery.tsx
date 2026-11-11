import { BatteryFullIcon, Icon } from "@barista/ui/icons"
import { MenubarItem } from "./menubar-item"

export function Battery() {
  return (
    <MenubarItem>
      <Icon icon={BatteryFullIcon} size={16} />
    </MenubarItem>
  )
}
