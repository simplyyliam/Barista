import { useEffect, useState } from "react"
import {
  commands,
  events,
  type Battery,
  type ClockSettings,
  type Volume,
} from "@/shared/bindings"

/** Keeps one system snapshot fresh: listen first, then ask once, replace on every event. */
function useSnapshot<T>(
  get: () => Promise<T>,
  listen: (handler: (value: T) => void) => Promise<() => void>,
): T | null {
  const [value, setValue] = useState<T | null>(null)

  useEffect(() => {
    let cancelled = false
    let gotEvent = false
    let stop: (() => void) | undefined
    void (async () => {
      const unlisten = await listen((next) => {
        gotEvent = true
        setValue(next)
      })
      if (cancelled) return unlisten()
      stop = unlisten
      const current = await get()
      if (!cancelled && !gotEvent) setValue(current)
    })()
    return () => {
      cancelled = true
      stop?.()
    }
    // get and listen are module-level functions: they never change.
  }, [])

  return value
}

export const useBattery = () =>
  useSnapshot<Battery>(commands.getBattery, (h) => events.batteryChanged.listen((e) => h(e.payload)))
export const useVolume = () =>
  useSnapshot<Volume>(commands.getVolume, (h) => events.volumeChanged.listen((e) => h(e.payload)))
export const useClockSettings = () =>
  useSnapshot<ClockSettings>(commands.getClockSettings, (h) =>
    events.clockSettingsChanged.listen((e) => h(e.payload)),
  )
