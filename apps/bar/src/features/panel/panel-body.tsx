import { useBattery, useClockSettings, useVolume } from "./hooks/use-system-state"

/** What a panel route shows. Only the live system values for now; the real designs come later. */
export function PanelBody({ name }: { name: string }) {
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
