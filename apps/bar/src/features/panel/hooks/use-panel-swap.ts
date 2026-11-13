import { useLayoutEffect, useRef } from "react"
import { useAnimate, useReducedMotion } from "motion/react"
import type { PanelOpen } from "@/shared/bindings"
import {
  PANEL_SWAP_FROM,
  PANEL_SWAP_TRANSITION,
  PANEL_SWAP_TRAVEL,
} from "@/shared/motion"

/**
 * The motion between two open panels. Rust moves the window to the new item at once, so the page
 * makes the swap readable: the new content starts offset towards where the old panel was, then
 * settles. Put the returned `scope` on the element around the panel content.
 *
 * Does nothing when a panel opens from closed, when the same panel is just re-measured, or for
 * panels that draw their own glass (the window itself is the card there).
 */
export function usePanelSwap(open: PanelOpen | null) {
  const [scope, animate] = useAnimate<HTMLDivElement>()
  const reduce = useReducedMotion()
  const before = useRef<{ key: string; center: number } | null>(null)

  useLayoutEffect(() => {
    if (!open) {
      before.current = null
      return
    }
    const key = `${open.name}|${open.anchor}`
    // Logical px; the rect is physical.
    const center = (open.rect.left + open.rect.right) / 2 / (open.scale ?? 1)
    const previous = before.current
    before.current = { key, center }
    const element = scope.current
    if (!previous || previous.key === key || open.chrome !== "none" || !element) return

    const dx = Math.max(-PANEL_SWAP_TRAVEL, Math.min(PANEL_SWAP_TRAVEL, previous.center - center))
    // Hide it first so the new content never shows one frame at full strength.
    element.style.opacity = "0"
    void animate(element, fromKeyframes(reduce ? 0 : dx, !!reduce), PANEL_SWAP_TRANSITION).then(() => {
      // No leftover filter: it would keep the content on its own layer.
      element.style.filter = ""
    })
  }, [open, animate, scope, reduce])

  return scope
}

/** Keyframes [from, to] per property; with reduced motion only the fade (and its blur) stays. */
function fromKeyframes(dx: number, reduce: boolean) {
  return {
    opacity: [PANEL_SWAP_FROM.opacity, 1],
    filter: [PANEL_SWAP_FROM.filter, "blur(0px)"],
    scale: reduce ? [1, 1] : [PANEL_SWAP_FROM.scale, 1],
    x: [dx, 0],
  }
}
