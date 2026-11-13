/** How many of the three Wi-Fi arcs a signal (0 to 100) lights. */
export function wifiStrength(signal: number | null | undefined): 0 | 1 | 2 | 3 {
  if (signal == null || !(signal > 0)) return 0
  if (signal > 66) return 3
  if (signal > 33) return 2
  return 1
}

/** How many of the three volume waves a level (0 to 1) lights: none at 0, then a third each. */
export function waveCount(level: number | null | undefined): 0 | 1 | 2 | 3 {
  if (level == null || !(level > 0)) return 0
  return Math.min(3, Math.ceil(level * 3)) as 1 | 2 | 3
}
