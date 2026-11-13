import { describe, expect, it } from "vitest"
import { formatDuration } from "./format-duration"

describe("formatDuration", () => {
  it("shows minutes under an hour", () => {
    expect(formatDuration(45 * 60)).toBe("45 min")
  })
  it("shows hours and minutes", () => {
    expect(formatDuration(2 * 3600 + 15 * 60)).toBe("2 h 15 min")
  })
  it("drops zero minutes", () => {
    expect(formatDuration(3 * 3600)).toBe("3 h")
  })
  it("says under a minute for tiny values", () => {
    expect(formatDuration(20)).toBe("under 1 min")
  })
})
