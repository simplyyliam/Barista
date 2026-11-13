import { describe, expect, it } from "vitest"
import { formatClock } from "./format-clock"

// Local time, so the test does not depend on the machine's time zone.
const at = (h: number, m: number) => new Date(2026, 10, 12, h, m)

describe("formatClock", () => {
  it("formats the date like macOS, without a comma", () => {
    expect(formatClock(at(13, 18), false).date).toBe("Thu Nov 12")
  })
  it("12 hour clock: no leading zero, AM or PM", () => {
    expect(formatClock(at(13, 18), false).time.replace(/\s/g, " ")).toBe("1:18 PM")
    expect(formatClock(at(0, 5), false).time.replace(/\s/g, " ")).toBe("12:05 AM")
  })
  it("24 hour clock: two digits, no AM or PM", () => {
    expect(formatClock(at(13, 18), true).time).toBe("13:18")
    expect(formatClock(at(9, 5), true).time).toBe("09:05")
  })
})
