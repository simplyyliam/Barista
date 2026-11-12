import { useBattery, useClockSettings, useVolume } from "./hooks/use-system-state"
import { NotificationsPanel } from "./notifications-panel"

/** What a panel route shows. `notifications` is the real design; the others show live values until theirs come. */
export function PanelBody({ name }: { name: string }) {
  return name === "notifications" ? <NotificationsPanel /> : <SystemValues name={name} />
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
