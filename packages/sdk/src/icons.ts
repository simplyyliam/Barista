// Icon helpers. The core of it (icon objects -> SVG) needs nothing. Cropping an icon to its real
// glyph (`fitToGlyph`) uses `@resvg/resvg-js`, an optional peer dependency that is loaded only when needed.
import { createRequire } from "node:module"

/** The bar is this high, in logical pixels (the macOS menu bar height). */
export const BAR_HEIGHT = 24
/** Nothing in the bar may be taller than this: the working area for icons. */
export const ICON_AREA = 22
/** A square icon's visible glyph is this big (width and height). The one number to tune. */
export const ICON_SIZE = 16
/** Wide icons (a battery) are sized by height: this tall, as wide as their shape needs. */
export const WIDE_ICON_HEIGHT = 12
/** Colour of icon objects when you give none (the bar is usually dark). */
export const DEFAULT_ICON_COLOR = "#f5f5f5"
/** Stroke width, in the icon's own 24-unit grid, when you give none. About 1.4 px on screen. */
export const DEFAULT_STROKE_WIDTH = 1.75

/**
 * An icon as data: a list of `[tag, attributes]` shapes on a 24-unit grid. This is exactly what
 * the `lucide` package exports for each icon and what `@hugeicons/core-free-icons` (and the other
 * Hugeicons packs) export, so you can pass them straight in.
 */
export type IconNode = ReadonlyArray<readonly [string, Readonly<Record<string, string | number | undefined>>]>

/** An icon object with options, for `setIcon({ icon, color })`. */
export interface IconSpec {
  icon: IconNode
  /** Any CSS colour. Default {@link DEFAULT_ICON_COLOR}. */
  color?: string
  /** Stroke width on the 24-unit grid. Default: the icon's own width, else {@link DEFAULT_STROKE_WIDTH}. */
  strokeWidth?: number
  /** Visible glyph height in logical pixels. Default {@link ICON_SIZE}; use {@link WIDE_ICON_HEIGHT} for wide shapes. */
  height?: number
}

/** Everything `item.setIcon` and `StatusItem.create({ icon })` accept. */
export type IconInput = Uint8Array | string | IconNode | IconSpec

const SHAPES = new Set(["path", "circle", "rect", "line", "polyline", "polygon", "ellipse", "g"])
const ATTRIBUTE = /^[a-zA-Z][a-zA-Z0-9:-]*$/

const kebab = (name: string) => name.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)
const escapeAttribute = (value: string) =>
  value.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;")

/** True for a bare icon object (`Clock01Icon`), as opposed to bytes, a string or an {@link IconSpec}. */
function isIconNode(value: unknown): value is IconNode {
  return Array.isArray(value)
}

/**
 * Turns an icon object into SVG text. Only plain shapes and plain attributes are allowed
 * (`path`, `circle`, `rect`...; no scripts, no event attributes), so an icon object from anywhere
 * cannot put markup into the bar. `currentColor` becomes `color`.
 *
 * @example
 * ```ts
 * import { Clock01Icon } from "@hugeicons/core-free-icons"
 * const svg = iconToSvg(Clock01Icon, { color: "#8b95a5" })
 * ```
 */
export function iconToSvg(
  icon: IconNode,
  options: { color?: string; strokeWidth?: number } = {},
): string {
  const color = options.color ?? DEFAULT_ICON_COLOR
  if (/["'<>&]/.test(color)) throw new TypeError(`Not a usable icon colour: ${JSON.stringify(color)}`)
  if (options.strokeWidth !== undefined && !(options.strokeWidth > 0 && options.strokeWidth <= 6)) {
    throw new RangeError(`strokeWidth must be above 0 and at most 6, got ${options.strokeWidth}`)
  }
  if (icon.length === 0 || icon.length > 200) throw new TypeError("An icon needs 1 to 200 shapes")
  const shapes = icon.map(([tag, attributes]) => {
    if (!SHAPES.has(tag)) throw new TypeError(`Icons may only contain ${[...SHAPES].join(", ")}, got <${tag}>`)
    const parts: string[] = []
    for (const [name, raw] of Object.entries(attributes)) {
      if (raw === undefined || name === "key") continue
      if (!ATTRIBUTE.test(name) || /^on/i.test(name)) throw new TypeError(`Bad attribute name ${JSON.stringify(name)}`)
      let value = String(raw)
      if (/^(stroke|fill)$/.test(name) && value === "currentColor") value = color
      if (name === "strokeWidth" && options.strokeWidth !== undefined) value = String(options.strokeWidth)
      parts.push(`${kebab(name)}="${escapeAttribute(value)}"`)
    }
    return `<${tag} ${parts.join(" ")}/>`
  })
  const stroke = options.strokeWidth ?? DEFAULT_STROKE_WIDTH
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="24" height="24" fill="none" ` +
    `stroke="${color}" stroke-width="${stroke}" stroke-linecap="round" stroke-linejoin="round">${shapes.join("")}</svg>`
  )
}

/** The part of resvg-js we use, so this file does not need its types installed. */
interface ResvgLike {
  getBBox(): { x: number; y: number; width: number; height: number } | undefined
  render(): { asPng(): Buffer }
}
type ResvgConstructor = new (svg: string, options?: { fitTo?: { mode: "height"; value: number } }) => ResvgLike

let resvg: ResvgConstructor | undefined

/** Thrown when `@resvg/resvg-js` is not installed. */
export class ResvgMissingError extends Error {
  constructor() {
    super("Fitting icons needs @resvg/resvg-js: run `npm install @resvg/resvg-js`")
    this.name = "ResvgMissingError"
  }
}

function loadResvg(): ResvgConstructor {
  if (resvg) return resvg
  try {
    resvg = (createRequire(import.meta.url)("@resvg/resvg-js") as { Resvg: ResvgConstructor }).Resvg
  } catch {
    throw new ResvgMissingError()
  }
  return resvg
}

const ROOT = /<svg\b([^>]*)>/i

function attribute(tag: string, name: string): string | undefined {
  return new RegExp(`\\s${name}\\s*=\\s*("([^"]*)"|'([^']*)')`, "i").exec(tag)?.slice(2, 4).find(Boolean)
}

function withoutAttribute(tag: string, name: string): string {
  return tag.replace(new RegExp(`\\s${name}\\s*=\\s*("[^"]*"|'[^']*')`, "ig"), "")
}

const round = (value: number, digits = 3) => Number(value.toFixed(digits))

/** The size the SVG draws in: its viewBox, else its width and height. */
function canvas(tag: string): { width: number; height: number } {
  const viewBox = attribute(tag, "viewBox")?.trim().split(/[\s,]+/).map(Number)
  if (viewBox?.length === 4 && viewBox.every(Number.isFinite) && viewBox[2]! > 0 && viewBox[3]! > 0) {
    return { width: viewBox[2]!, height: viewBox[3]! }
  }
  const width = Number.parseFloat(attribute(tag, "width") ?? "")
  const height = Number.parseFloat(attribute(tag, "height") ?? "")
  if (width > 0 && height > 0) return { width, height }
  throw new TypeError("The SVG needs a viewBox, or a width and a height")
}

/**
 * Crops an SVG to what it really draws and sizes it by height. Icon sets such as Lucide and
 * Hugeicons draw inside a 24-unit box with about 2 units of empty border, so a 16 px box shows
 * a glyph only about 13 px big. This measures the glyph's real bounds (stroke included), makes
 * those the `viewBox`, and sets `height` to `targetHeight` and `width` from the aspect ratio:
 * a square icon comes out square, a wide one (a battery) comes out wide.
 *
 * `targetHeight` is in logical pixels, at most {@link ICON_AREA}. Returns SVG text you can pass
 * to `item.setIcon`. Throws a `TypeError` for an SVG it cannot read or one that draws nothing,
 * and a {@link ResvgMissingError} if `@resvg/resvg-js` is not installed.
 *
 * @example
 * ```ts
 * import { fitToGlyph, ICON_SIZE } from "@baristabar/sdk/icons"
 * item.setIcon(fitToGlyph(fs.readFileSync("clock.svg", "utf8"), ICON_SIZE))
 * ```
 */
export function fitToGlyph(svg: string, targetHeight: number = ICON_SIZE): string {
  if (!(targetHeight > 0) || targetHeight > ICON_AREA) {
    throw new RangeError(`targetHeight must be above 0 and at most ${ICON_AREA}, got ${targetHeight}`)
  }
  if (!ROOT.test(svg)) throw new TypeError("Not an SVG: no <svg> element found")
  // resvg refuses an SVG without xmlns, so make sure the file has one before anything else.
  if (!/\sxmlns\s*=/.test(ROOT.exec(svg)![0])) {
    svg = svg.replace(/<svg\b/i, '<svg xmlns="http://www.w3.org/2000/svg"')
  }
  const tag = ROOT.exec(svg)![0]
  const box = canvas(tag)
  const Resvg = loadResvg()

  // Measure on a canvas where one unit is one pixel, so the numbers are in viewBox units.
  let measuring = withoutAttribute(withoutAttribute(tag, "width"), "height")
  measuring = measuring.replace(/<svg\b/i, `<svg width="${box.width}" height="${box.height}"`)
  if (!/viewBox/i.test(measuring)) {
    measuring = measuring.replace(/<svg\b/i, `<svg viewBox="0 0 ${box.width} ${box.height}"`)
  }
  let bounds
  try {
    bounds = new Resvg(svg.replace(tag, measuring)).getBBox()
  } catch (error) {
    throw new TypeError(`Could not read the SVG: ${(error as Error).message}`, { cause: error })
  }
  if (!bounds || !(bounds.width > 0) || !(bounds.height > 0)) {
    throw new TypeError("The SVG draws nothing, so there is no glyph to fit")
  }

  // The bounds are in the SVG's own units, so they become the viewBox as they are. The glyph then
  // touches the box edge: nothing is clipped, because the box is exactly what is drawn.
  const { x, y, width, height } = bounds
  const outWidth = (width / height) * targetHeight
  const root = withoutAttribute(withoutAttribute(withoutAttribute(tag, "viewBox"), "width"), "height").replace(
    /<svg\b/i,
    `<svg viewBox="${round(x)} ${round(y)} ${round(width)} ${round(height)}" width="${round(outWidth, 2)}" height="${targetHeight}"`,
  )
  return svg.replace(tag, root)
}

/** Options for {@link toPng}. */
export interface ToPngOptions {
  /** Height in logical pixels, as for {@link fitToGlyph}. Default {@link ICON_SIZE}. */
  height?: number
  /** The monitor scale: 1.5 = 150%. The PNG is `height * scale` pixels high. Default 2. */
  scale?: number
}

/**
 * Fits the SVG with {@link fitToGlyph}, then draws it as a PNG at `height * scale` pixels (18 px
 * at 150% for a 12 px icon), so it stays sharp on a scaled screen. SVG icons are already sharp
 * at any scale; use this when you want a PNG.
 *
 * @example
 * ```ts
 * item.setIcon(toPng(svgText, { height: 16, scale: 1.5 }))
 * ```
 */
export function toPng(svg: string, options: ToPngOptions = {}): Buffer {
  const height = options.height ?? ICON_SIZE
  const scale = options.scale ?? 2
  if (!(scale > 0) || scale > 8) throw new RangeError(`scale must be above 0 and at most 8, got ${scale}`)
  const fitted = fitToGlyph(svg, height)
  const pixels = Math.max(1, Math.round(height * scale))
  return new (loadResvg())(fitted, { fitTo: { mode: "height", value: pixels } }).render().asPng()
}

let warned = false

/**
 * Turns whatever `setIcon` was given into bytes or a string the bar understands. An icon object
 * becomes an SVG, cropped to its glyph when `@resvg/resvg-js` is installed (without it the icon
 * still works but looks smaller, and a warning says how to fix that, once).
 * @internal
 */
export function resolveIcon(icon: IconInput): Uint8Array | string {
  if (icon instanceof Uint8Array) return icon
  // SVG text is fine too (an empty string and `data:` URLs pass through unchanged).
  if (typeof icon === "string") return icon.trimStart().startsWith("<") ? Buffer.from(icon) : icon
  const spec: IconSpec = isIconNode(icon) ? { icon } : icon
  const svg = iconToSvg(spec.icon, {
    ...(spec.color !== undefined && { color: spec.color }),
    ...(spec.strokeWidth !== undefined && { strokeWidth: spec.strokeWidth }),
  })
  try {
    return Buffer.from(fitToGlyph(svg, spec.height ?? ICON_SIZE))
  } catch (error) {
    if (!(error instanceof ResvgMissingError)) throw error
    if (!warned) {
      warned = true
      console.warn(`[barista] ${error.message}. Until then icons keep their empty border and look small.`)
    }
    return Buffer.from(svg)
  }
}
