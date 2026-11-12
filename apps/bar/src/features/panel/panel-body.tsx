import { useBattery, useClockSettings, useVolume } from "./hooks/use-system-state"
import { NotificationsPanel } from "./notifications-panel"

/**
 * What a panel route shows. `name` is `null` while the panel is closed. The notifications route is
 * kept mounted even then (out of the layout, invisible) so it opens with everything loaded; the
 * other routes show live values until their designs come.
 */
export function PanelBody({ name }: { name: string | null }) {
  const active = name === "notifications"
  return (
    <>
      <div className={active ? undefined : "invisible absolute top-0 left-0"} aria-hidden={!active}>
        <NotificationsPanel active={active} />
      </div>
      {name !== null && !active && <SystemValues name={name} />}
    </>
  )
}

function SystemValues({ name }: { name: string }) {
  const battery = useBattery()
  const volume = useVolume()
  const clock = useClockSettings()

  const batteryText = !battery
    ? "..."
    : battery.present
      ? `${battery.percent ?? "?"}%${battery.pluggedIn ? ", plugged in" : ""}`
      : "none"
  const volumeText = !volume
    ? "..."
    : volume.available
      ? `${Math.round((volume.level ?? 0) * 100)}%${volume.muted ? " (muted)" : ""}`
      : "no output"

  return (
    <div className="flex flex-col gap-2 p-4 text-[13px]">
      <div className="font-medium capitalize">{name.replace(/-/g, " ")}</div>
      <div>Battery: {batteryText}</div>
      <div>Volume: {volumeText}</div>
      <div>Clock: {!clock ? "..." : `${clock.hour24 ? "24" : "12"} hour, ${clock.timeZone}`}</div>
    </div>
  )
}
