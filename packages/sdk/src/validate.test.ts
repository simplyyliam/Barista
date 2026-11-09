import { describe, expect, it } from "vitest"
import { MAX_ICON_BYTES } from "./protocol.js"
import { assertItemId, assertLabel, assertTooltip, toIconDataUrl } from "./validate.js"

/** A PNG signature plus an IHDR of width x height, then `extra` filler bytes. */
const png = (extra = 0, width = 16, height = 16) => {
  const header = Buffer.alloc(16)
  header.writeUInt32BE(13, 0)
  header.write("IHDR", 4, "latin1")
  header.writeUInt32BE(width, 8)
  header.writeUInt32BE(height, 12)
  return Buffer.concat([Buffer.from("89504e470d0a1a0a", "hex"), header, Buffer.alloc(extra)])
}

describe("validation", () => {
  it("accepts good ids and rejects bad ones", () => {
    for (const id of ["a", "counter", "a-b_c.d9", "x".repeat(64)])
      expect(() => assertItemId(id)).not.toThrow()
    for (const id of ["", "A", "has space", "a/b", "x".repeat(65)])
      expect(() => assertItemId(id)).toThrow(TypeError)
  })

  it("limits text length and control characters", () => {
    expect(() => assertLabel("x".repeat(128))).not.toThrow()
    expect(() => assertLabel("x".repeat(129))).toThrow(/longer/)
    expect(() => assertLabel("a\nb")).toThrow(/control/)
    expect(() => assertTooltip("x".repeat(513))).toThrow(/longer/)
  })

  it("turns PNG bytes into a data URL", () => {
    expect(toIconDataUrl(png(4))).toMatch(/^data:image\/png;base64,/)
  })

  it("turns SVG bytes into a data URL", () => {
    const url = toIconDataUrl(Buffer.from("<svg xmlns='http://www.w3.org/2000/svg'/>"))
    expect(url).toMatch(/^data:image\/svg\+xml;base64,/)
  })

  it("rejects other bytes and oversized icons", () => {
    expect(() => toIconDataUrl(Buffer.from("hello"))).toThrow(/PNG or an SVG/)
    expect(() => toIconDataUrl(png(MAX_ICON_BYTES))).toThrow(/limit/)
  })

  it("passes valid data URLs through and rejects other strings", () => {
    expect(toIconDataUrl("")).toBe("")
    const url = `data:image/png;base64,${png(4).toString("base64")}`
    expect(toIconDataUrl(url)).toBe(url)
    expect(() => toIconDataUrl("https://example.com/a.png")).toThrow(/data: URL/)
  })

  it("checks the content of a data URL like it checks bytes", () => {
    const notPng = `data:image/png;base64,${Buffer.from("hello").toString("base64")}`
    expect(() => toIconDataUrl(notPng)).toThrow(/PNG or an SVG/)
    const svgAsPng = `data:image/png;base64,${Buffer.from("<svg/>").toString("base64")}`
    expect(() => toIconDataUrl(svgAsPng)).toThrow(/says image\/png/)
    const big = `data:image/png;base64,${png(MAX_ICON_BYTES).toString("base64")}`
    expect(() => toIconDataUrl(big)).toThrow(/bigger|limit/)
    expect(() => toIconDataUrl(`data:image/png;base64,${"A".repeat(900_000)}`)).toThrow(/bigger/)
  })

  it("caps PNG dimensions", () => {
    expect(() => toIconDataUrl(png(0, 1024, 1024))).not.toThrow()
    expect(() => toIconDataUrl(png(0, 1025, 16))).toThrow(/pixels/)
    expect(() => toIconDataUrl(png(0, 30000, 30000))).toThrow(/pixels/)
    expect(() => toIconDataUrl(png(0, 0, 16))).toThrow(/pixels/)
    expect(() => toIconDataUrl(Buffer.from("89504e470d0a1a0a", "hex"))).toThrow(/pixels/)
  })
})
