import { useLayoutEffect, useRef } from "react"
import { useAnimate, useReducedMotion } from "motion/react"
import type { PanelOpen } from "@/shared/bindings"
import { PANEL_SWAP_FROM, PANEL_SWAP_TRANSITION } from "@/shared/motion"

/**
 * The motion between two open panels. Rust moves the window to the new item at once, so the page
 * makes the swap readable: the new content fades in and grows slightly, from the side the old panel
 * was on. Put the returned `scope` on the element around the panel content.
 *
 * Only opacity and scale, never a move or a blur: the window is exactly the size of the panel, so
 * anything leaving its box is clipped in a hard line. The scale origin is clamped inside the box
 * for the same reason (shrinking towards a point inside never leaves the box).
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
    const scale = open.scale ?? 1
    // Logical px; the rect is physical.
    const left = open.rect.left / scale
    const width = (open.rect.right - open.rect.left) / scale
    const center = left + width / 2
    const previous = before.current
    before.current = { key, center }
    const element = scope.current
    if (!previous || previous.key === key || open.chrome !== "none" || !element || width <= 0) return

    const originX = Math.max(0, Math.min(100, ((previous.center - left) / width) * 100))
    element.style.transformOrigin = `${originX}% 0%`
    // Hide it first so the new content never shows one frame at full strength.
    element.style.opacity = "0"
    void animate(
      element,
      { opacity: [PANEL_SWAP_FROM.opacity, 1], scale: reduce ? [1, 1] : [PANEL_SWAP_FROM.scale, 1] },
      PANEL_SWAP_TRANSITION,
    )
  }, [open, animate, scope, reduce])

  return scope
}
