import {
  commands,
  events,
  type Battery,
  type ClockSettings,
  type Mode,
  type Volume,
  type Wifi_Deserialize,
} from "@/shared/bindings"
import { useSnapshot } from "@/shared/hooks/use-snapshot"

export const useBattery = () =>
  useSnapshot<Battery>(commands.getBattery, (h) => events.batteryChanged.listen((e) => h(e.payload)))
export const useVolume = () =>
  useSnapshot<Volume>(commands.getVolume, (h) => events.volumeChanged.listen((e) => h(e.payload)))
export const useClockSettings = () =>
  useSnapshot<ClockSettings>(commands.getClockSettings, (h) =>
    events.clockSettingsChanged.listen((e) => h(e.payload)),
  )
export const useWifi = () =>
  useSnapshot<Wifi_Deserialize>(commands.getWifi, (h) => events.wifiChanged.listen((e) => h(e.payload)))
/** Who draws the control center: "barista" (ours) or "windows" (Quick Settings). Rule 11 switch. */
export const useControlCenterMode = () =>
  useSnapshot<Mode>(commands.getControlCenterMode, (h) =>
    events.controlCenterModeChanged.listen((e) => h(e.payload)),
  )
