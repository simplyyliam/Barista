import { useCallback, useEffect, useMemo, useRef } from "react"
import { isAimedAtSubmenu, type AimTarget, type Point } from "../lib/navigation"

/** A held move waits this long for the pointer to keep going before it counts. */
const SETTLE_MS = 150
/** Holding never lasts longer than this in total, however the pointer moves. */
const MAX_HOLD_MS = 500
/** Slower than this (px/ms, about 80 px/s) the user is choosing a row, not travelling. */
const DELIBERATE_SPEED = 0.08

type Sample = Point & { t: number }

/**
 * Hover intent for submenus (see docs/research/submenu-intent.md). While a submenu is open,
 * the pointer heading for it passes over other rows; those rows must not take over.
 *
 * A move is held (the row under the pointer is ignored for now) when it is
 *  - aimed at the submenu: its direction lies inside the cone to the submenu's near corners,
 *    plus 15 degrees (React Aria's angle test), and
 *  - fast enough to be travelling (Framer's speed test), not a slow, deliberate move.
 * A held move is applied once the pointer settles, or after 500 ms. One stray move is forgiven
 * (a shaky hand); the second in a row switches rows at once.
 *
 * `move(sample, apply)` is called for every pointer move over a row.
 */
export function useSafeTriangle() {
  const target = useRef<AimTarget | null>(null)
  const last = useRef<Sample | null>(null)
  const heldSince = useRef<number | null>(null)
  const strays = useRef(0)
  const timer = useRef<number | undefined>(undefined)

  const cancel = useCallback(() => {
    window.clearTimeout(timer.current)
    timer.current = undefined
    heldSince.current = null
    strays.current = 0
  }, [])

  useEffect(() => cancel, [cancel])

  const setPlacement = useCallback(
    (next: AimTarget | null) => {
      target.current = next
      if (!next) cancel()
    },
    [cancel],
  )

  const move = useCallback((sample: Sample, apply: () => void) => {
    const aim = target.current
    const previous = last.current
    last.current = sample

    let hold = false
    if (aim && previous) {
      const dt = Math.max(sample.t - previous.t, 1)
      const speed = Math.hypot(sample.x - previous.x, sample.y - previous.y) / dt
      const aimed = isAimedAtSubmenu(previous, sample, aim)
      if (aimed) {
        strays.current = 0
        hold = speed >= DELIBERATE_SPEED
      } else {
        strays.current += 1
        hold = strays.current < 2
      }
      if (hold && heldSince.current !== null && sample.t - heldSince.current > MAX_HOLD_MS) hold = false
    }

    window.clearTimeout(timer.current)
    if (!hold) {
      heldSince.current = null
      strays.current = 0
      apply()
      return
    }
    heldSince.current ??= sample.t
    timer.current = window.setTimeout(() => {
      heldSince.current = null
      strays.current = 0
      apply()
    }, SETTLE_MS)
  }, [])

  /** The pointer moved over the row that is already active: only remember where it is. */
  const track = useCallback(
    (sample: Sample) => {
      last.current = sample
      cancel()
    },
    [cancel],
  )

  return useMemo(() => ({ setPlacement, move, track, cancel }), [setPlacement, move, track, cancel])
}
