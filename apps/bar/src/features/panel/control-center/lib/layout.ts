import type { ControlCenterPrefs_Serialize } from "@/shared/bindings"

/** What the control center shows until Rust has sent the saved layout (the design: App.png). */
export const DEFAULT_CONTROL_CENTER: ControlCenterPrefs_Serialize = {
  tiles: [
    { id: "wifi", size: "wide", visible: true },
    { id: "airplane", size: "small", visible: true },
    { id: "night-light", size: "small", visible: true },
    { id: "bluetooth", size: "small", visible: true },
    { id: "hotspot", size: "small", visible: true },
    { id: "battery-saver", size: "wide", visible: true },
  ],
  userCard: true,
  brightness: true,
  volume: true,
  nowPlaying: true,
  quickActions: ["dark-mode", "timer", "calculator", "camera", "screenshot"],
}

/**
 * How many rows a 4 column grid needs for tiles of these widths (1 or 2 columns), filled left to
 * right like CSS grid does: a wide tile that does not fit in the row's last column moves to the next row.
 */
export function gridRows(spans: number[], columns = 4): number {
  let rows = spans.length > 0 ? 1 : 0
  let used = 0
  for (const span of spans) {
    if (used + span > columns) {
      rows += 1
      used = 0
    }
    used += span
  }
  return rows
}
