// CPU and RAM from Node's own `os` module: cheap enough to read once a second.
import os from "node:os"

export interface CpuTimes {
  idle: number
  total: number
}

/** The cores' time counters added together (they only grow). */
export function cpuTimes(cpus: os.CpuInfo[] = os.cpus()): CpuTimes {
  let idle = 0
  let total = 0
  for (const { times } of cpus) {
    idle += times.idle
    total += times.user + times.nice + times.sys + times.idle + times.irq
  }
  return { idle, total }
}

/** Percent of CPU used between two readings, 0 to 100. The first reading has nothing to compare. */
export function cpuPercent(before: CpuTimes | undefined, now: CpuTimes): number {
  if (!before) return 0
  const total = now.total - before.total
  if (total <= 0) return 0
  const busy = total - (now.idle - before.idle)
  return Math.min(100, Math.max(0, Math.round((busy / total) * 100)))
}

/** Percent busy per core between two readings (the same maths as `cpuPercent`, one core at a time). */
export function corePercents(before: os.CpuInfo[] | undefined, now: os.CpuInfo[]): number[] {
  return now.map((core, i) => {
    const prev = before?.[i]
    return cpuPercent(prev && cpuTimes([prev]), cpuTimes([core]))
  })
}

export interface Memory {
  /** Percent in use, 0 to 100. */
  percent: number
  usedGb: number
  totalGb: number
}

export function memory(total = os.totalmem(), free = os.freemem()): Memory {
  const used = total - free
  const gb = (bytes: number) => Math.round((bytes / 1024 ** 3) * 10) / 10
  return { percent: Math.round((used / total) * 100), usedGb: gb(used), totalGb: gb(total) }
}

/** A fixed-size history: the newest value last, the oldest dropped. */
export function push(history: number[], value: number, size = 60): number[] {
  history.push(value)
  while (history.length > size) history.shift()
  return history
}
