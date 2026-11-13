import { describe, expect, it } from "vitest"
import { clamp01, toPercentStep, valueForKey, valueFromPointer } from "./slider-math"

describe("valueFromPointer", () => {
  it("puts the head's edge under the pointer", () => {
    expect(valueFromPointer(150, 100, 200)).toBe(0.25)
    expect(valueFromPointer(100, 100, 200)).toBe(0)
    expect(valueFromPointer(300, 100, 200)).toBe(1)
  })
  it("clamps a pointer outside the slider", () => {
    expect(valueFromPointer(40, 100, 200)).toBe(0)
    expect(valueFromPointer(900, 100, 200)).toBe(1)
  })
  it("survives a zero width", () => {
    expect(valueFromPointer(150, 100, 0)).toBe(0)
  })
})

describe("clamp01 and toPercentStep", () => {
  it("clamps and refuses NaN", () => {
    expect(clamp01(-1)).toBe(0)
    expect(clamp01(2)).toBe(1)
    expect(clamp01(NaN)).toBe(0)
  })
  it("rounds to whole percent", () => {
    expect(toPercentStep(0.3049)).toBe(0.3)
    expect(toPercentStep(0.305)).toBe(0.31)
  })
})

describe("valueForKey", () => {
  it("moves 5 % with arrows and 10 % with page keys, inside 0 to 1", () => {
    expect(valueForKey("ArrowRight", 0.5)).toBe(0.55)
    expect(valueForKey("ArrowDown", 0.5)).toBe(0.45)
    expect(valueForKey("PageUp", 0.95)).toBe(1)
    expect(valueForKey("PageDown", 0.05)).toBe(0)
  })
  it("jumps to the ends and ignores other keys", () => {
    expect(valueForKey("Home", 0.5)).toBe(0)
    expect(valueForKey("End", 0.5)).toBe(1)
    expect(valueForKey("a", 0.5)).toBeNull()
  })
})
