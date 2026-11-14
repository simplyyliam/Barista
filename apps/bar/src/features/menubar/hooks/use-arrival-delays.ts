import { useRef } from "react"
import { staggerDelay } from "@/shared/motion"

/**
 * Items that show up in the same update get a growing delay, so a bar start (every app's item
 * arriving at once) is a small wave instead of a pop. A single new item gets none.
 */
export function useArrivalDelays(ids: string[]) {
  const delays = useRef(new Map<string, number>())
  const known = delays.current
  let fresh = 0
  for (const id of ids) {
    if (!known.has(id)) known.set(id, staggerDelay(fresh++))
  }
  for (const id of known.keys()) {
    if (!ids.includes(id)) known.delete(id)
  }
  return (id: string) => known.get(id) ?? 0
}
