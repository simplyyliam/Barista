import { useCallback, useEffect, useMemo, useRef } from "react"
import { inTriangle, type Point } from "../lib/navigation"
import type { SubmenuPlacement } from "./use-menu-state"

/** A held move waits this long for the pointer to keep going before it counts. */
const SETTLE_MS = 150
/** The safe triangle never holds for longer than this in total. */
const MAX_HOLD_MS = 400

/**
 * The "safe triangle": moving the pointer diagonally from the row that opened a submenu toward
 * the submenu passes over other rows. While the pointer is inside the triangle (its last
 * position, and the submenu's two near corners) those rows are ignored, so the submenu stays.
 *
 * `move(point, apply)` is called for every pointer move over a row. It either calls `apply` now,
 * or holds it and calls it once the pointer settles (or leaves the triangle).
 */
export function useSafeTriangle() {
  const placement = useRef<SubmenuPlacement | null>(null)
  const last = useRef<Point | null>(null)
  const heldSince = useRef<number | null>(null)
  const timer = useRef<number | undefined>(undefined)

  const cancel = useCallback(() => {
    window.clearTimeout(timer.current)
    timer.current = undefined
    heldSince.current = null
  }, [])

  useEffect(() => cancel, [cancel])

  const setPlacement = useCallback(
    (next: SubmenuPlacement | null) => {
      placement.current = next
      if (!next) cancel()
    },
    [cancel],
  )

  const move = useCallback(
    (point: Point, apply: () => void) => {
      const target = placement.current
      const previous = last.current
      last.current = point

      let hold = false
      if (target && previous) {
        const edgeX = target.side === "right" ? target.x : target.x + target.width
        hold = inTriangle(
          point,
          previous,
          { x: edgeX, y: target.y },
          { x: edgeX, y: target.y + target.height },
        )
      }

      const now = performance.now()
      if (hold && heldSince.current !== null && now - heldSince.current > MAX_HOLD_MS) hold = false

      window.clearTimeout(timer.current)
      if (!hold) {
        heldSince.current = null
        apply()
        return
      }
      heldSince.current ??= now
      timer.current = window.setTimeout(() => {
        heldSince.current = null
        apply()
      }, SETTLE_MS)
    },
    [],
  )

  /** The pointer moved over the row that is already active: only remember where it is. */
  const track = useCallback(
    (point: Point) => {
      last.current = point
      cancel()
    },
    [cancel],
  )

  return useMemo(() => ({ setPlacement, move, track, cancel }), [setPlacement, move, track, cancel])
}
