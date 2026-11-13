import { useBattery, useClockSettings, useVolume } from "@/shared/hooks/use-system-state"
import { ControlCenterPanel } from "./control-center/control-center-panel"
import { NotificationsPanel } from "./notifications-panel"

/**
 * What a panel route shows. `name` is `null` while the panel is closed. The notifications route is
 * and the control center route are kept mounted even then (out of the layout, invisible) so they
 * open with everything loaded; the other routes show live values until their designs come.
 */
export function PanelBody({ name, warm = false }: { name: string | null; warm?: boolean }) {
  const active = name === "notifications"
  const controlCenter = name === "control-center"
  return (
    <>
      <div
        // Closed: out of the layout and invisible. Warming (startup, window hidden): drawn and in the
        // layout, so it is painted once and its real size can be measured.
        className={active || warm ? undefined : "invisible absolute top-0 left-0"}
        aria-hidden={!active}
      >
        <NotificationsPanel active={active} />
      </div>
      <div
        // Always mounted like the notifications route, so the control center opens fully loaded.
        className={controlCenter ? undefined : "invisible absolute top-0 left-0"}
        aria-hidden={!controlCenter}
      >
        <ControlCenterPanel active={controlCenter} />
      </div>
      {name !== null && !active && !controlCenter && <SystemValues name={name} />}
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
