import { useEffect, useRef, useState } from "react"
import {
  commands,
  events,
  type AirplaneMode,
  type Bluetooth,
  type Brightness,
  type Hotspot,
  type Media_Deserialize,
  type NightLight,
  type ThemeMode,
  type User_Deserialize,
  type Wifi_Deserialize,
} from "@/shared/bindings"
import { useSnapshot } from "@/shared/hooks/use-snapshot"

// One live snapshot per system service: read once, then replaced by every `...Changed` event.
export const useWifi = () =>
  useSnapshot<Wifi_Deserialize>(commands.getWifi, (h) => events.wifiChanged.listen((e) => h(e.payload)))
export const useBluetooth = () =>
  useSnapshot<Bluetooth>(commands.getBluetooth, (h) => events.bluetoothChanged.listen((e) => h(e.payload)))
export const useAirplane = () =>
  useSnapshot<AirplaneMode>(commands.getAirplaneMode, (h) =>
    events.airplaneModeChanged.listen((e) => h(e.payload)),
  )
export const useHotspot = () =>
  useSnapshot<Hotspot>(commands.getHotspot, (h) => events.hotspotChanged.listen((e) => h(e.payload)))
export const useBrightness = () =>
  useSnapshot<Brightness>(commands.getBrightness, (h) =>
    events.brightnessChanged.listen((e) => h(e.payload)),
  )
export const useUser = () =>
  useSnapshot<User_Deserialize>(commands.getUser, (h) => events.userChanged.listen((e) => h(e.payload)))
export const useMedia = () =>
  useSnapshot<Media_Deserialize>(commands.getMedia, (h) => events.mediaChanged.listen((e) => h(e.payload)))
export const useNightLight = () =>
  useSnapshot<NightLight>(commands.getNightLight, (h) =>
    events.nightLightChanged.listen((e) => h(e.payload)),
  )
export const useThemeMode = () =>
  useSnapshot<ThemeMode>(commands.getThemeMode, (h) =>
    events.themeModeChanged.listen((e) => h(e.payload)),
  )

/**
 * A slider's value while the user drags it. Rust only hears about it once per frame (a drag fires
 * far more often than that), and the snapshot from Rust is ignored until the drag ends so the
 * thumb never jumps back.
 */
export function useDragValue(current: number | null, send: (value: number) => void) {
  const [dragging, setDragging] = useState<number | null>(null)
  const pending = useRef<number | null>(null)
  const frame = useRef(0)
  const sendRef = useRef(send)
  sendRef.current = send

  useEffect(() => () => cancelAnimationFrame(frame.current), [])

  const change = (value: number) => {
    setDragging(value)
    pending.current = value
    if (frame.current) return
    frame.current = requestAnimationFrame(() => {
      frame.current = 0
      if (pending.current !== null) sendRef.current(pending.current)
    })
  }

  // The last value always goes out, then the snapshot takes over again.
  const end = (value: number) => {
    cancelAnimationFrame(frame.current)
    frame.current = 0
    pending.current = null
    sendRef.current(value)
    setDragging(null)
  }

  return { value: dragging ?? current ?? 0, change, end }
}
