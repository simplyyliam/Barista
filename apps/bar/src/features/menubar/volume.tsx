import { useVolume } from "@/shared/hooks/use-system-state"
import { VolumeIcon } from "./icons"
import { MenubarItem } from "./menubar-item"

type VolumeViewProps = { level: number; muted: boolean; open?: boolean }

/** The volume pill from plain values. */
export function VolumeView({ level, muted, open }: VolumeViewProps) {
  return (
    <MenubarItem variant="icon" open={open} className="cursor-default" aria-label="Volume">
      <VolumeIcon level={level} muted={muted} />
    </MenubarItem>
  )
}

/** Output volume and mute from the volume service. Not shown without an output device. */
export function Volume() {
  const volume = useVolume()
  if (!volume?.available) return null
  return <VolumeView level={volume.level ?? 0} muted={volume.muted} />
}
