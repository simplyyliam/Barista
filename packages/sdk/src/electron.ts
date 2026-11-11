// Electron adapter for popovers. It imports nothing from Electron: you pass in your
// BrowserWindow (and optionally `screen`), so the SDK never depends on Electron.
import type { Placement, PopoverWindow } from "./popover.js"

interface Rect {
  x: number
  y: number
  width: number
  height: number
}

/** The parts of Electron's `BrowserWindow` the adapter uses. A real one fits. */
export interface ElectronWindowLike {
  setBounds(bounds: Rect): void
  show(): void
  focus(): void
  hide(): void
  on(event: "closed", listener: () => void): unknown
  webContents: {
    on(
      event: "before-input-event",
      listener: (event: unknown, input: { type: string; key: string }) => void,
    ): unknown
  }
}

/** The part of Electron's `screen` module the adapter uses. */
export interface ElectronScreenLike {
  screenToDipRect(window: null, rect: Rect): Rect
}

/** Options for {@link electronPopover}. */
export interface ElectronPopoverOptions {
  /**
   * Electron's `screen` module. With it the bar's physical-pixel rect is converted exactly, even
   * with monitors at different scales. Without it the adapter divides by the monitor's scale,
   * which is only right on a single-scale setup.
   */
  screen?: ElectronScreenLike
}

/**
 * Turns an Electron `BrowserWindow` into a {@link PopoverWindow}: it moves the window to the
 * rect the bar sends, shows it, focuses it, and closes it on Esc.
 *
 * Create the window hidden and frameless: `show: false, frame: false, skipTaskbar: true,
 * alwaysOnTop: true`. Do not hide it on blur: the bar hides it when the user clicks away.
 *
 * @example
 * ```ts
 * import { BrowserWindow, screen } from "electron"
 * import * as BA from "@barista/sdk"
 * import { electronPopover } from "@barista/sdk/electron"
 *
 * const win = new BrowserWindow({ show: false, frame: false, skipTaskbar: true, alwaysOnTop: true })
 * const item = await BA.StatusItem.create({ id: "note", label: "Note" })
 * item.setPopover({ window: electronPopover(win, { screen }), width: 320, height: 380 })
 * ```
 */
export function electronPopover(
  win: ElectronWindowLike,
  options: ElectronPopoverOptions = {},
): PopoverWindow {
  const listeners = new Set<() => void>()
  const closed = () => {
    for (const listener of [...listeners]) listener()
  }
  win.on("closed", closed)
  win.webContents.on("before-input-event", (_event, input) => {
    if (input.type === "keyDown" && input.key === "Escape") {
      win.hide()
      closed()
    }
  })
  return {
    show(placement) {
      win.setBounds(toBounds(placement, options.screen))
      win.show()
      win.focus()
    },
    hide: () => win.hide(),
    onClose: (listener) => {
      listeners.add(listener)
    },
  }
}

/** Physical pixels from the bar -> Electron's scaled (DIP) pixels. */
export function toBounds(placement: Placement, screen?: ElectronScreenLike): Rect {
  const physical = {
    x: placement.x,
    y: placement.y,
    width: placement.width,
    height: placement.height,
  }
  const dip = screen
    ? screen.screenToDipRect(null, physical)
    : {
        x: physical.x / placement.scale,
        y: physical.y / placement.scale,
        width: physical.width / placement.scale,
        height: physical.height / placement.scale,
      }
  return {
    x: Math.round(dip.x),
    y: Math.round(dip.y),
    width: Math.round(dip.width),
    height: Math.round(dip.height),
  }
}
