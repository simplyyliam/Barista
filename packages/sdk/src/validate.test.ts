import { describe, expect, it } from "vitest"
import { MAX_ICON_BYTES } from "./protocol.js"
import { assertItemId, assertLabel, assertTooltip, toIconDataUrl } from "./validate.js"

const png = (extra = 0) =>
  Buffer.concat([Buffer.from("89504e470d0a1a0a", "hex"), Buffer.alloc(extra)])

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

  it("passes data URLs through and rejects other strings", () => {
    expect(toIconDataUrl("")).toBe("")
    const url = "data:image/png;base64,AAAA"
    expect(toIconDataUrl(url)).toBe(url)
    expect(() => toIconDataUrl("https://example.com/a.png")).toThrow(/data: URL/)
  })
})
