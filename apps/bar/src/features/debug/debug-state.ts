/** Made-up values for the bar preview in the debug window (nothing here talks to Rust). */
export type DebugState = {
  batteryPresent: boolean
  batteryPercent: number
  charging: boolean
  showBatteryLabel: boolean
  showPercentInside: boolean
  wifiStrength: 0 | 1 | 2 | 3
  wifiOff: boolean
  volume: number
  muted: boolean
  hour24: boolean
  /** Draws every pill as open (pressed). Hover is real: move the mouse over the bar. */
  allOpen: boolean
  /** --menubar-height, set only on the previews. */
  height: number
}

export const DEFAULT_DEBUG_STATE: DebugState = {
  batteryPresent: true,
  batteryPercent: 72,
  charging: false,
  showBatteryLabel: false,
  showPercentInside: false,
  wifiStrength: 3,
  wifiOff: false,
  volume: 0.6,
  muted: false,
  hour24: false,
  allOpen: false,
  height: 28,
}
