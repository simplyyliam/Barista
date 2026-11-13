import type { ReactNode } from "react"
import { commands, type AppVolume } from "@/shared/bindings"
import { useAppVolumes, useOutputDevices, useVolume } from "@/shared/hooks/use-system-state"
import {
  Icon,
  MusicNote01Icon,
  Tick02Icon,
  VolumeHighIcon,
  VolumeMute01Icon,
} from "@barista/ui/icons"
import { cn } from "cn"
import { useDragValue } from "./control-center/hooks/use-control-center"
import { VolumeSlider } from "./control-center/control-center-panel"
import { SystemSlider } from "./control-center/system-slider"

/** Same width as the control center (--spacing-panel-w). */
export const VOLUME_PANEL_WIDTH = 300
/** A first guess; Rust remembers the measured size after the first open. */
export const VOLUME_PANEL_HEIGHT = 500

function Block({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-1.5 rounded-panel-child bg-cc-tile p-3 text-cc-fg">
      <span className="px-0.5 text-[11px] font-medium tracking-wide text-cc-fg-muted uppercase">{title}</span>
      {children}
    </div>
  )
}

/** One app: its icon, its name and a slider (the icon on the slider mutes it). */
function AppRow({ app }: { app: AppVolume }) {
  const drag = useDragValue(app.level, (level) => void commands.setAppVolume(app.id, level))
  return (
    <div className={cn("flex items-center gap-2.5", !app.playing && "opacity-60")}>
      {app.icon ? (
        <img src={app.icon} alt="" draggable={false} className="size-5 shrink-0 rounded-[4px]" />
      ) : (
        <Icon icon={MusicNote01Icon} size={18} className="shrink-0 text-cc-fg-muted" />
      )}
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate text-[12px] leading-tight">{app.name}</span>
        <SystemSlider
          label={app.name}
          icon={app.muted ? VolumeMute01Icon : VolumeHighIcon}
          value={app.muted ? 0 : drag.value}
          // Moving a muted app up unmutes it, like the master slider does.
          onChange={(level) => {
            if (app.muted && level > 0) void commands.setAppMuted(app.id, false)
            drag.change(level)
          }}
          onCommit={drag.end}
          onIconClick={() => void commands.setAppMuted(app.id, !app.muted)}
        />
      </div>
    </div>
  )
}

/**
 * The `volume` route, opened from the volume item in the bar: the master slider with its device,
 * one slider per app that is using audio (Windows' volume mixer), the output devices to switch
 * between, and a link to the Windows sound settings.
 */
export function VolumePanel() {
  const volume = useVolume()
  const outputs = useOutputDevices()
  const apps = useAppVolumes()

  return (
    <div className="flex w-panel-w flex-col">
      <section className="flex flex-col gap-cc-gap rounded-panel bg-cc-card p-cc-pad">
        <Block title="Output">
          <span className="truncate px-0.5 text-[12px] text-cc-fg-muted">{volume?.deviceName}</span>
          <VolumeSlider />
        </Block>

        <Block title="Apps">
          {!apps || apps.apps.length === 0 ? (
            <span className="px-0.5 py-2 text-[12px] text-cc-fg-muted">No app is using audio</span>
          ) : (
            <div className="flex max-h-[168px] flex-col gap-3 overflow-y-auto py-1">
              {apps.apps.map((app) => (
                <AppRow key={app.id} app={app} />
              ))}
            </div>
          )}
        </Block>

        {outputs && outputs.devices.length > 0 && (
          <Block title="Output device">
            <div className="flex max-h-[108px] flex-col overflow-y-auto">
              {outputs.devices.map((device) => (
                <button
                  key={device.id}
                  type="button"
                  onClick={() => void commands.setOutputDevice(device.id)}
                  className="flex h-9 shrink-0 items-center gap-2 rounded-[max(0px,calc(var(--radius-panel-child)-var(--spacing-cc-pad)))] px-2 text-left text-[13px] outline-none hover:bg-cc-tile-hover focus-visible:ring-2 focus-visible:ring-cc-fg/60"
                >
                  <span className="min-w-0 flex-1 truncate">{device.name}</span>
                  {device.default && <Icon icon={Tick02Icon} size={16} className="shrink-0" />}
                </button>
              ))}
            </div>
          </Block>
        )}

        <button
          type="button"
          onClick={() => void commands.openSettings("sound")}
          className="h-9 rounded-panel-child bg-cc-tile px-3 text-left text-[13px] text-cc-fg outline-none hover:bg-cc-tile-hover focus-visible:ring-2 focus-visible:ring-cc-fg/60"
        >
          Sound settings…
        </button>
      </section>
    </div>
  )
}
