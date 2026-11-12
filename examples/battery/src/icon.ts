// The battery icon uses the supplied rounded macOS-style outline, with a dynamic charge fill
// and larger charging / plugged-in marks. It is cropped and sized to a 12 px glyph height.
import { fitToGlyph, WIDE_ICON_HEIGHT } from "@barista/sdk/icons"

const OUTLINE = "#f5f5f5"
const MARK_OUTLINE = "#1c1c1e"

// The supplied 24 x 24 battery silhouette.
const BATTERY_PATH =
  "M8 18H16.5C16.9644 18 17.1966 18 17.3916 17.9743C18.7378 17.7971 19.7971 16.7378 19.9743 15.3916C20 15.1966 20 14.9644 20 14.5C21.1046 14.5 22 13.6046 22 12.5V11.5C22 10.3954 21.1046 9.5 20 9.5C20 9.03558 20 8.80337 19.9743 8.60842C19.7971 7.26222 18.7378 6.2029 17.3916 6.02567C17.1966 6 16.9644 6 16.5 6H8C5.17157 6 3.75736 6 2.87868 6.87868C2 7.75736 2 9.17157 2 12C2 14.8284 2 16.2426 2.87868 17.1213C3.75736 18 5.17157 18 8 18Z"

// Keep the fill inset from the outline and stop it before the terminal.
const FILL = { x: 3.6, y: 7.6, maxWidth: 14.8, height: 8.8, radius: 1.5 }
const CENTER_X = 11

export function batteryIcon(
  percentage: number,
  charging: boolean,
  pluggedIn: boolean,
): Buffer {
  // Red when low, white when charging / plugged in, and light grey on battery.
  const fillColor =
    percentage <= 15
      ? "#ef4444"
      : charging || pluggedIn
        ? "#ffffff"
        : "#f5f5f5"

  // Keep a visible rounded dot at very low non-zero percentages.
  const fillWidth =
    percentage > 0
      ? Math.max(FILL.radius * 2, (FILL.maxWidth * percentage) / 100)
      : 0

  const level =
    fillWidth > 0
      ? `<rect x="${FILL.x}" y="${FILL.y}" width="${fillWidth.toFixed(2)}" height="${FILL.height}" rx="${FILL.radius}" fill="${fillColor}"/>`
      : ""

  // White marks with a dark edge for visibility over filled and empty areas.
  const markStyle = `fill="#ffffff" stroke="${MARK_OUTLINE}" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"`
  const c = CENTER_X

  const bolt = charging
    ? `<path d="M${c + 1.8} 7.2 L${c - 4.2} 12 H${c - 0.5} L${c - 1.8} 16.6 L${c + 4.6} 11 H${c + 0.7} Z" ${markStyle}/>`
    : ""

  // Larger plug with wider prongs and a clearly visible handle.
  // const plug =
  //   pluggedIn && !charging
  //     ? `<path d="M${c - 3.4} 7.3 V9.9 H${c - 4.2} V14.1 Q${c - 4.2} 15.7 ${c - 2.6} 15.7 H${c - 1} V16.8 H${c + 1} V15.7 H${c + 2.6} Q${c + 4.2} 15.7 ${c + 4.2} 14.1 V9.9 H${c + 3.4} V7.3 H${c + 1.7} V9.9 H${c - 1.7} V7.3 Z" ${markStyle}/>`
  //     : ""

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24">
    <path
      d="${BATTERY_PATH}"
      fill="none"
      stroke="${OUTLINE}"
      stroke-width="1.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    ${level}${bolt}
  </svg>`

  try {
    return Buffer.from(fitToGlyph(svg, WIDE_ICON_HEIGHT))
  } catch (error) {
    // Without @resvg/resvg-js the icon still works, it just keeps its small empty border.
    console.warn("Could not fit the battery icon:", (error as Error).message)
    return Buffer.from(svg)
  }
}