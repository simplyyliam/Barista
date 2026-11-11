import { describe, expect, it } from "vitest"
import { fitToGlyph, ICON_AREA, ICON_SIZE, toPng, WIDE_ICON_HEIGHT } from "./icons.js"

// A 24-box icon with the usual 2 units of border: a circle of radius 9, stroke 2 (spans 2..22).
const CIRCLE =
  '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="2"><circle cx="12" cy="12" r="9"/></svg>'

function attrs(svg: string) {
  const viewBox = /viewBox="([^"]+)"/.exec(svg)![1]!.split(" ").map(Number)
  return {
    viewBox,
    width: Number(/\swidth="([^"]+)"/.exec(svg)![1]),
    height: Number(/\sheight="([^"]+)"/.exec(svg)![1]),
  }
}

describe("fitToGlyph", () => {
  it("crops the empty border of a 24-box icon and sizes it by height", () => {
    const { viewBox, width, height } = attrs(fitToGlyph(CIRCLE, 16))
    // The glyph (stroke included) spans 2..22, so that is exactly the new box.
    expect(viewBox).toEqual([2, 2, 20, 20])
    expect(height).toBe(16)
    expect(width).toBe(16)
  })

  it("keeps a wide shape wide: the width follows the aspect ratio", () => {
    const wide =
      '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 20" fill="none" stroke="#000" stroke-width="2"><rect x="1" y="5" width="38" height="10"/></svg>'
    const { width, height } = attrs(fitToGlyph(wide, 12))
    expect(height).toBe(12)
    // 40 x 12 units (stroke included) at 12 px tall is 40 px wide.
    expect(width).toBe(40)
  })

  it("counts the stroke in the bounds", () => {
    const thin = CIRCLE.replace('stroke-width="2"', 'stroke-width="0.5"')
    const thick = CIRCLE.replace('stroke-width="2"', 'stroke-width="6"')
    expect(attrs(fitToGlyph(thick, 16)).viewBox[2]).toBeGreaterThan(attrs(fitToGlyph(thin, 16)).viewBox[2]!)
  })

  it("works without a viewBox (width and height only) and with a viewBox that does not start at 0", () => {
    const plain = CIRCLE.replace(' viewBox="0 0 24 24"', "")
    expect(attrs(fitToGlyph(plain, 16)).width).toBeCloseTo(16, 0)
    const shifted = CIRCLE.replace('viewBox="0 0 24 24"', 'viewBox="100 50 24 24"').replace(
      'cx="12" cy="12"',
      'cx="112" cy="62"',
    )
    const { viewBox } = attrs(fitToGlyph(shifted, 16))
    expect(viewBox).toEqual([102, 52, 20, 20])
  })

  it("is idempotent in size: fitting a fitted icon changes nothing visible", () => {
    const once = attrs(fitToGlyph(CIRCLE, 16))
    const twice = attrs(fitToGlyph(fitToGlyph(CIRCLE, 16), 16))
    expect(twice.width).toBeCloseTo(once.width, 0)
    expect(twice.height).toBe(16)
  })

  it("adds xmlns when it is missing so the result is a valid image", () => {
    expect(fitToGlyph(CIRCLE.replace(' xmlns="http://www.w3.org/2000/svg"', ""), 16)).toContain("xmlns=")
  })

  it("rejects things that are not a usable icon", () => {
    expect(() => fitToGlyph("<div/>")).toThrow(TypeError)
    expect(() => fitToGlyph('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"></svg>')).toThrow(
      /draws nothing/,
    )
    expect(() => fitToGlyph('<svg xmlns="http://www.w3.org/2000/svg"><circle r="3"/></svg>')).toThrow(
      /viewBox/,
    )
    expect(() => fitToGlyph(CIRCLE, ICON_AREA + 1)).toThrow(RangeError)
    expect(() => fitToGlyph(CIRCLE, 0)).toThrow(RangeError)
  })

  it("uses the shared size constants by default", () => {
    expect(attrs(fitToGlyph(CIRCLE)).height).toBe(ICON_SIZE)
    expect(WIDE_ICON_HEIGHT).toBeLessThan(ICON_SIZE)
    expect(ICON_SIZE).toBeLessThanOrEqual(ICON_AREA)
  })
})

describe("toPng", () => {
  const size = (png: Buffer) => ({ width: png.readUInt32BE(16), height: png.readUInt32BE(20) })

  it("draws at height times scale pixels (18 px at 150% for a 12 px icon)", () => {
    const png = toPng(CIRCLE, { height: 12, scale: 1.5 })
    expect(png.subarray(1, 4).toString()).toBe("PNG")
    expect(size(png).height).toBe(18)
    expect(size(png).width).toBe(18)
  })

  it("rejects an absurd scale", () => {
    expect(() => toPng(CIRCLE, { scale: 20 })).toThrow(RangeError)
  })
})

// What @hugeicons/core-free-icons exports for Clock01Icon, and the same idea from `lucide`.
const CLOCK = [
  ["circle", { cx: "12", cy: "12", r: "10", stroke: "currentColor", strokeWidth: "1.5", key: "0" }],
  [
    "path",
    { d: "M12 8V12L14 14", stroke: "currentColor", strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: "1.5", key: "1" },
  ],
] as const

describe("iconToSvg", () => {
  it("turns a Hugeicons or Lucide icon object into SVG", async () => {
    const { iconToSvg } = await import("./icons.js")
    const svg = iconToSvg(CLOCK, { color: "#8b95a5" })
    expect(svg).toContain('viewBox="0 0 24 24"')
    expect(svg).toContain('stroke-linecap="round"') // camelCase became kebab-case
    expect(svg).toContain('stroke="#8b95a5"') // currentColor became the colour
    expect(svg).not.toContain("currentColor")
    expect(svg).not.toContain("key=")
    // And it is a real image: it fits like any other SVG.
    expect(fitToGlyph(svg, 16)).toContain('height="16"')
  })

  it("lets strokeWidth override the icon's own", async () => {
    const { iconToSvg } = await import("./icons.js")
    expect(iconToSvg(CLOCK, { strokeWidth: 2 })).toContain('stroke-width="2"')
    expect(iconToSvg(CLOCK, { strokeWidth: 2 })).not.toContain('stroke-width="1.5"')
    expect(() => iconToSvg(CLOCK, { strokeWidth: 0 })).toThrow(RangeError)
  })

  it("refuses anything that is not a plain shape, so an icon object cannot inject markup", async () => {
    const { iconToSvg } = await import("./icons.js")
    expect(() => iconToSvg([["script", {}]])).toThrow(/may only contain/)
    expect(() => iconToSvg([["path", { onload: "x()" }]])).toThrow(/Bad attribute/)
    expect(() => iconToSvg([["path", { 'd" onload="x': "M0 0" }]])).toThrow(/Bad attribute/)
    expect(() => iconToSvg(CLOCK, { color: 'red" onload="x' })).toThrow(TypeError)
    expect(() => iconToSvg([])).toThrow(TypeError)
    // Values are escaped, not trusted.
    expect(iconToSvg([["path", { d: 'M0 0"><script>' }]])).not.toContain("<script>")
  })
})

describe("resolveIcon", () => {
  it("crops an icon object to its glyph and sizes it, so it looks bigger than the raw 24-box", async () => {
    const { resolveIcon } = await import("./icons.js")
    const bytes = resolveIcon(CLOCK) as Uint8Array
    const svg = Buffer.from(bytes).toString("utf8")
    expect(svg).toMatch(/height="16"/)
    const viewBox = /viewBox="([^"]+)"/.exec(svg)![1]!.split(" ").map(Number)
    expect(viewBox[2]).toBeLessThan(24) // the empty border is gone
  })

  it("honours height and passes bytes, text and data URLs through", async () => {
    const { resolveIcon } = await import("./icons.js")
    expect(Buffer.from(resolveIcon({ icon: CLOCK, height: 12 }) as Uint8Array).toString()).toMatch(/height="12"/)
    const bytes = new Uint8Array([1, 2])
    expect(resolveIcon(bytes)).toBe(bytes)
    expect(resolveIcon("")).toBe("")
    expect(resolveIcon("data:image/png;base64,AAAA")).toBe("data:image/png;base64,AAAA")
    expect(Buffer.from(resolveIcon(CIRCLE) as Uint8Array).toString()).toBe(CIRCLE) // SVG text -> bytes
  })
})
