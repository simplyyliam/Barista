import { useSyncExternalStore } from "react"
import { commands, events } from "@/shared/bindings"

/**
 * The tone of the WALLPAPER behind the bar, measured by Rust.
 * "light" wallpaper = dark content, "dark" wallpaper = light content.
 * It is also written to <html data-tone="...">, which switches the tokens in styles/tokens.css.
 */
export type BarTone = "light" | "dark"

// Light is also the CSS default, so nothing flashes before Rust answers.
let tone: BarTone = "light"
const listeners = new Set<() => void>()

function setTone(next: BarTone) {
  document.documentElement.dataset.tone = next
  if (next === tone) return
  tone = next
  listeners.forEach((l) => l())
}

/**
 * Starts following the bar tone. Call once, before rendering (main.tsx does).
 *
 * @example
 * void startBarTone()
 */
export async function startBarTone() {
  setTone(tone)
  // Listen first, then read, so a change in between is not lost.
  await events.barAppearanceChanged.listen((e) => setTone(e.payload.tone))
  setTone((await commands.getBarAppearance()).tone)
}

/** The current tone outside React. */
export const getBarTone = () => tone

/**
 * The current tone inside React; re-renders when it flips.
 *
 * @example
 * const tone = useBarTone() // "light" | "dark"
 */
export function useBarTone(): BarTone {
  return useSyncExternalStore(
    (cb) => {
      listeners.add(cb)
      return () => listeners.delete(cb)
    },
    () => tone,
  )
}
