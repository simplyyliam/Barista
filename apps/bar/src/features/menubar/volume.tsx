import { Icon, VolumeHighIcon } from "@barista/ui/icons"
import { MenubarItem } from "./menubar-item"

export function Volume() {
  return (
    <MenubarItem>
      <Icon icon={VolumeHighIcon} size={16} />
    </MenubarItem>
  )
}
