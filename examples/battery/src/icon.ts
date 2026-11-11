// The battery icon: custom-drawn (the fill level and the charging / plugged marks change), then cropped
// to its real shape and sized by height with `fitToGlyph`, so it is 12 px tall and about 25 wide.
import { fitToGlyph, WIDE_ICON_HEIGHT } from "@barista/sdk/icons"

// The drawing grid is 29 x 14 units, so the body is 2:1 like macOS (25 x 12 px after fitting). The body
// width is a number, not a stretch of a square shape, so the corners stay round. Stroke 1.7 units is
// about 1.45 px once the icon is 12 px tall.
const OUTLINE = "#f5f5f5"
const MARK_OUTLINE = "#1c1c1e"
const BODY = { x: 0.85, y: 0.85, width: 24.8, height: 12.3, radius: 3.3, stroke: 1.7 }
const FILL = { x: 2.8, y: 2.8, maxWidth: 20.9, height: 8.4, radius: 1.6 }
/** Where the marks sit: the middle of the inside of the body. */
const CENTER_X = 13.25

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

  // The marks use the whole height of the inside (like macOS): white with a dark edge, so they read on
  // the fill and on the empty part alike. They are positioned around CENTER_X.
  const markStyle = `fill="#ffffff" stroke="${MARK_OUTLINE}" stroke-width=".8" stroke-linejoin="round"`
  const c = CENTER_X
  const bolt = charging
    ? `<path d="M${c + 1.9} 2.2 ${c - 4.4} 7.7h3.9l-1.6 4.1 6.6-5.8h-4.1z" ${markStyle}/>`
    : ""
  const plug =
    pluggedIn && !charging
      ? `<g ${markStyle}>
           <rect x="${c - 2.4}" y="2.3" width="1.4" height="3.4" rx=".5"/>
           <rect x="${c + 1}" y="2.3" width="1.4" height="3.4" rx=".5"/>
           <rect x="${c - 3.5}" y="5.1" width="7" height="4.8" rx="1.9"/>
           <rect x="${c - 0.7}" y="9.5" width="1.4" height="2.4" rx=".5"/>
         </g>`
      : ""

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 29 14" width="29" height="14">
      <rect x="${BODY.x}" y="${BODY.y}" width="${BODY.width}" height="${BODY.height}" rx="${BODY.radius}"
        fill="none" stroke="${OUTLINE}" stroke-opacity=".9" stroke-width="${BODY.stroke}"/>
      <rect x="27.2" y="4.5" width="1.8" height="5" rx=".9" fill="${OUTLINE}" fill-opacity=".6"/>
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
