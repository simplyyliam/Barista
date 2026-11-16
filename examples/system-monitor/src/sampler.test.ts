import { describe, expect, it } from "vitest"
import { cpuPercent, cpuTimes, memory, push } from "./sampler.ts"

describe("sampler", () => {
  it("adds up the cores", () => {
    const core = (idle: number, user: number) => ({
      model: "x",
      speed: 1,
      times: { user, nice: 0, sys: 0, idle, irq: 0 },
    })
    expect(cpuTimes([core(10, 5), core(20, 5)])).toEqual({ idle: 30, total: 40 })
  })

  it("is the busy share between two readings", () => {
    expect(cpuPercent({ idle: 0, total: 0 }, { idle: 75, total: 100 })).toBe(25)
    expect(cpuPercent({ idle: 50, total: 100 }, { idle: 150, total: 200 })).toBe(0)
  })

  it("has nothing to say without a first reading or a standing clock", () => {
    expect(cpuPercent(undefined, { idle: 1, total: 2 })).toBe(0)
    expect(cpuPercent({ idle: 1, total: 2 }, { idle: 1, total: 2 })).toBe(0)
  })

  it("reads memory as used over total", () => {
    const gb = 1024 ** 3
    expect(memory(16 * gb, 4 * gb)).toEqual({ percent: 75, usedGb: 12, totalGb: 16 })
  })

  it("keeps only the newest values", () => {
    const h = [1, 2, 3]
    expect(push(h, 4, 3)).toEqual([2, 3, 4])
  })
})
