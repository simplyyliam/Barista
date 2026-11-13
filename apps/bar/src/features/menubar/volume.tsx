import { useVolume } from "@/shared/hooks/use-system-state"
import { VolumeIcon } from "./icons"
import { MenubarItem } from "./menubar-item"

/** Output volume and mute. Not shown without an output device. */
export function Volume() {
  const volume = useVolume()
  if (!volume?.available) return null

  return (
    <MenubarItem className="cursor-default" aria-label="Volume">
      <VolumeIcon level={volume.level ?? 0} muted={volume.muted} />
    </MenubarItem>
  )
}
