import { describe, expect, it } from "vitest"
import { waveCount, wifiStrength } from "./levels"

describe("wifiStrength", () => {
  it("maps 0-100 to 0-3 arcs", () => {
    expect([null, 0, 1, 33, 34, 66, 67, 100].map(wifiStrength)).toEqual([0, 0, 1, 1, 2, 2, 3, 3])
  })
})

describe("waveCount", () => {
  it("lights none at 0 and a third more each step", () => {
    expect([null, 0, 0.01, 0.33, 0.34, 0.66, 0.67, 1].map(waveCount)).toEqual([0, 0, 1, 1, 2, 2, 3, 3])
  })
  it("ignores NaN", () => {
    expect(waveCount(NaN)).toBe(0)
  })
})
