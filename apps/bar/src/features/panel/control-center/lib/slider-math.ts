/** Keeps a slider value in 0 to 1; anything that is not a number counts as 0. */
export const clamp01 = (value: number) => Math.min(1, Math.max(0, Number.isFinite(value) ? value : 0))

/**
 * The value under the pointer: the head's right edge sits exactly where the pointer is, so what
 * you grab is what moves (no offset between the cursor and the bar).
 */
export function valueFromPointer(clientX: number, left: number, width: number) {
  if (!(width > 0)) return 0
  return clamp01((clientX - left) / width)
}

/** What goes to Rust: whole percent (the head itself follows the pointer smoothly). */
export const toPercentStep = (value: number) => Math.round(clamp01(value) * 100) / 100

/** Keyboard: arrows move 5 %, Page keys 10 %, Home and End the ends. `null` = not our key. */
export function valueForKey(key: string, value: number): number | null {
  switch (key) {
    case "ArrowRight":
    case "ArrowUp":
      return toPercentStep(value + 0.05)
    case "ArrowLeft":
    case "ArrowDown":
      return toPercentStep(value - 0.05)
    case "PageUp":
      return toPercentStep(value + 0.1)
    case "PageDown":
      return toPercentStep(value - 0.1)
    case "Home":
      return 0
    case "End":
      return 1
    default:
      return null
  }
}
