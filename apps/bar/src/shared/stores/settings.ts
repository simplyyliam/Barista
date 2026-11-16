import { useSyncExternalStore } from "react"
import { commands, events, type Settings_Serialize } from "@/shared/bindings"

/** The settings as Rust sends them (every field present). */
export type Settings = Settings_Serialize

/**
 * The saved settings (Rust holds them, ADR 0029). One listener per window: components read them
 * with `useSettings()`, and every change comes from the `settingsChanged` event.
 */
let current: Settings | null = null
const listeners = new Set<() => void>()
let started = false

function set(next: Settings) {
  current = next
  for (const listener of listeners) listener()
}

function start() {
  if (started) return
  started = true
  let gotEvent = false
  void events.settingsChanged.listen((e) => {
    gotEvent = true
    set(e.payload as Settings)
  })
  void commands.getSettings().then((settings) => {
    if (!gotEvent) set(settings as Settings)
  })
}

/** The settings, or `null` until Rust has answered (a moment after the window opens). */
export function useSettings(): Settings | null {
  start()
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
    () => current,
  )
}
