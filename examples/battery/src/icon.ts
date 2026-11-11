// The battery icon: custom-drawn (the fill level and the charging / plugged marks change), then cropped
// to its real shape and sized by height with `fitToGlyph`, so it is 12 px tall and about 24 wide.
import { fitToGlyph, WIDE_ICON_HEIGHT } from "@barista/sdk/icons"

// The drawing grid is 28 x 14 units, so the body is 2:1 like macOS. The body width is a number,
// not a stretch of a square shape, so the corners stay round. Stroke 1.7 units is about 1.45 px
// once the icon is 12 px tall.
const OUTLINE = "#f5f5f5"
const MARK_OUTLINE = "#1c1c1e"
const BODY = { x: 0.85, y: 0.85, width: 23.8, height: 12.3, radius: 3.3, stroke: 1.7 }
const FILL = { x: 2.8, y: 2.8, maxWidth: 19.9, height: 8.4, radius: 1.6 }

export function batteryIcon(percentage: number, charging: boolean, pluggedIn: boolean): Buffer {
  // White when plugged in or charging (it was blue), red when low, light grey on battery.
  const fillColor = percentage <= 15 ? "#ef4444" : charging || pluggedIn ? "#ffffff" : "#f5f5f5"
  // Never narrower than a rounded dot, or a nearly empty battery would draw nothing.
  const fillWidth =
    percentage > 0 ? Math.max(FILL.radius * 2, (FILL.maxWidth * percentage) / 100) : 0

  const level =
    fillWidth > 0
      ? `<rect x="${FILL.x}" y="${FILL.y}" width="${fillWidth.toFixed(2)}" height="${FILL.height}" rx="${FILL.radius}" fill="${fillColor}"/>`
      : ""

  // Marks are white with a thin dark edge so they read on the fill and on the empty part alike.
  const markStyle = `fill="#ffffff" stroke="${MARK_OUTLINE}" stroke-width=".7" stroke-linejoin="round"`
  const bolt = charging
    ? `<path d="M14.7 2.3 8.8 7.6h3.5l-1.4 4.1 6-5.6h-3.7z" ${markStyle}/>`
    : ""
  const plug =
    pluggedIn && !charging
      ? `<g ${markStyle}>
           <rect x="11.2" y="3.2" width=".95" height="2.4" rx=".4"/>
           <rect x="13.5" y="3.2" width=".95" height="2.4" rx=".4"/>
           <rect x="10.2" y="5.4" width="5.1" height="3.4" rx="1.2"/>
           <rect x="12.3" y="8.6" width=".95" height="2.6" rx=".4"/>
         </g>`
      : ""

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 28 14" width="28" height="14">
      <rect x="${BODY.x}" y="${BODY.y}" width="${BODY.width}" height="${BODY.height}" rx="${BODY.radius}"
        fill="none" stroke="${OUTLINE}" stroke-opacity=".9" stroke-width="${BODY.stroke}"/>
      <rect x="26.2" y="4.5" width="1.8" height="5" rx=".9" fill="${OUTLINE}" fill-opacity=".6"/>
      ${level}${bolt}${plug}
    </svg>`

  try {
    return Buffer.from(fitToGlyph(svg, WIDE_ICON_HEIGHT))
  } catch (error) {
    // Without @resvg/resvg-js the icon still works, it just keeps its small empty border.
    console.warn("Could not fit the battery icon:", (error as Error).message)
    return Buffer.from(svg)
  }
}
