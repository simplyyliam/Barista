import { MAX_POPOVER_SIZE, MIN_POPOVER_SIZE, type Tone } from "./protocol.js"

/**
 * Where the bar wants your popover window, in physical pixels (what Windows calls pixels, not
 * the scaled ones your UI framework uses). The bar already placed it under the icon and kept it
 * on screen: apply it as it is.
 */
export interface Placement {
  x: number
  y: number
  width: number
  height: number
  /** The monitor's scale: 1.5 = 150%. */
  scale: number
  /** The id of the monitor the popover is on. */
  monitor: string
  /** `"dark"` when the bar is dark, so your window can match. */
  tone: Tone
}

/**
 * A window the bar can show as a popover. The SDK knows nothing about window frameworks: you
 * hand it anything that can do these three things. `@barista/sdk/electron` has a ready-made one
 * for Electron.
 *
 * Esc and window-close handling belong to you: when your window closes itself, call the
 * listener passed to `onClose` (the Electron adapter does this for Esc).
 *
 * @example
 * ```ts
 * const popover: BA.PopoverWindow = {
 *   show: (p) => myWindow.moveTo(p.x, p.y, p.width, p.height).show().focus(),
 *   hide: () => myWindow.hide(),
 *   onClose: (listener) => myWindow.on("escape", listener),
 * }
 * ```
 */
export interface PopoverWindow {
  /** Move, size and show the window, and give it keyboard focus. May be async. */
  show(placement: Placement): void | Promise<void>
  /** Hide the window. The bar asks for this; you do not need to report it back. */
  hide(): void
  /** Call `listener` when the window closes by itself (Esc, the user closed it). */
  onClose(listener: () => void): void
}

/** Options for {@link StatusItem.setPopover}. */
export interface PopoverOptions {
  /** The window to show. */
  window: PopoverWindow
  /** Size you would like, in logical pixels (48 to 1024). The bar keeps it on screen. */
  width: number
  height: number
}

/** Throws a `TypeError` unless `width` and `height` are whole numbers the bar accepts. */
export function assertPopoverSize(width: number, height: number): void {
  for (const [name, value] of [
    ["width", width],
    ["height", height],
  ] as const) {
    if (!Number.isInteger(value) || value < MIN_POPOVER_SIZE || value > MAX_POPOVER_SIZE) {
      throw new TypeError(
        `Popover ${name} must be a whole number from ${MIN_POPOVER_SIZE} to ${MAX_POPOVER_SIZE}, got ${String(value)}`,
      )
    }
  }
}
