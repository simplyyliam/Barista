import { type ComponentProps } from "react"
import { cn } from "cn"
import { commands } from "@/shared/bindings"
import { useControlCenterMode, useVolume } from "@/shared/hooks/use-system-state"
import { useOpenPanel } from "@/features/panel"
import { VOLUME_PANEL_HEIGHT, VOLUME_PANEL_WIDTH } from "@/features/panel/volume-panel"
import { VolumeIcon } from "./icons"
import { useItemBounds } from "./hooks/use-item-bounds"
import { ITEM_APPEAR, MenubarItem } from "./menubar-item"

const ITEM_ID = "system/volume"

type VolumeViewProps = ComponentProps<typeof MenubarItem> & { level: number; muted: boolean; open?: boolean }

/** The volume pill from plain values (the debug window feeds it made-up ones). */
export function VolumeView({ level, muted, open, className, ...props }: VolumeViewProps) {
  return (
    <MenubarItem variant="icon" open={open} className={cn("cursor-default", className)} aria-label="Volume" {...ITEM_APPEAR} {...props}>
      <VolumeIcon level={level} muted={muted} />
    </MenubarItem>
  )
}

/**
 * Output volume and mute from the volume service. Click opens the volume panel under the icon
 * (master slider, one slider per app, output devices), or Windows' Quick Settings when the
 * control center is set to "windows". Not shown without an output device.
 */
export function Volume() {
  const volume = useVolume()
  const mode = useControlCenterMode()
  const open = useOpenPanel()
  const available = volume?.available === true

  // The panel opens under the item, so Rust needs to know where it is drawn. The item shows up
  // once the device is known, so the key changes then and the position is sent again.
  useItemBounds(available ? ITEM_ID : "")

  if (!volume || !available) return null

  const click = () => {
    if (mode === "windows") {
      void commands.openWindowsControlCenter()
    } else {
      void commands.openPanel("volume", ITEM_ID, VOLUME_PANEL_WIDTH, VOLUME_PANEL_HEIGHT, "none")
    }
  }

  const isOpen = open?.anchor === ITEM_ID
  return (
    <VolumeView
      level={volume.level ?? 0}
      muted={volume.muted}
      open={isOpen}
      role="button"
      tabIndex={0}
      aria-expanded={isOpen}
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
