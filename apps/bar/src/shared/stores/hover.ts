import { useSyncExternalStore } from "react"

/**
 * Bar pills draw their hover tint from their own pointer events, not CSS `:hover`. When the pointer
 * leaves the bar for another of our windows (a panel, a menu, a popover), the bar's webview never
 * hears that it left, and `:hover` stays on until the pointer comes back. `resetHover()` makes every
 * pill forget it was hovered; the next pointer move over a pill sets it again.
 */
let epoch = 0
const listeners = new Set<() => void>()

export function resetHover() {
  epoch += 1
  listeners.forEach((listener) => listener())
}

function subscribe(listener: () => void) {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

/** Changes every time `resetHover()` runs. */
export function useHoverEpoch() {
  return useSyncExternalStore(subscribe, () => epoch)
}
