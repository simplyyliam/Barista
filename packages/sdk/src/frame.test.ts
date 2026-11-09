import { describe, expect, it } from "vitest"
import { FrameDecoder, encodeFrame } from "./frame.js"
import { MAX_MESSAGE_BYTES } from "./protocol.js"

describe("frames", () => {
  it("round-trips a message", () => {
    const decoder = new FrameDecoder()
    expect(decoder.push(encodeFrame({ type: "x", n: 1 }))).toEqual([{ type: "x", n: 1 }])
  })

  it("writes a 4-byte little-endian length first", () => {
    const frame = encodeFrame({ a: 1 })
    expect(frame.readUInt32LE(0)).toBe(frame.length - 4)
    expect(frame.subarray(4).toString()).toBe('{"a":1}')
  })

  it("reassembles a message split across chunks, byte by byte", () => {
    const decoder = new FrameDecoder()
    const frame = encodeFrame({ hello: "wörld" })
    const out: unknown[] = []
    for (let i = 0; i < frame.length; i++) out.push(...decoder.push(frame.subarray(i, i + 1)))
    expect(out).toEqual([{ hello: "wörld" }])
  })

  it("returns several messages from one chunk", () => {
    const decoder = new FrameDecoder()
    const both = Buffer.concat([encodeFrame({ n: 1 }), encodeFrame({ n: 2 })])
    expect(decoder.push(both)).toEqual([{ n: 1 }, { n: 2 }])
  })

  it("rejects zero and oversized lengths", () => {
    for (const length of [0, MAX_MESSAGE_BYTES + 1]) {
      const header = Buffer.alloc(4)
      header.writeUInt32LE(length)
      expect(() => new FrameDecoder().push(header)).toThrow(/not allowed/)
    }
  })

  it("refuses to send a message over the limit", () => {
    expect(() => encodeFrame({ big: "x".repeat(MAX_MESSAGE_BYTES) })).toThrow(/limit/)
  })
})
