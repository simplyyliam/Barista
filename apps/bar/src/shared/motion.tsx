import type { ReactNode } from "react"
import { AnimatePresence, MotionConfig, motion, type Transition } from "motion/react"
import { cn } from "cn"

/**
 * One motion language for the whole app (ADR 0023, docs/research/ux-motion-and-taste.md).
 * Rules in short: hover is instant, a press dips to 0.96, things that change meaning fade + scale +
 * blur, exits are faster than enters, nothing loops, and the panel windows never animate open.
 */

/** Strong ease-out: starts fast, so the interface answers at once. Never ease-in for UI. */
export const EASE_OUT = [0.23, 1, 0.32, 1] as const

/** A press dips to 0.96 (never below 0.95). Full-width rows use PRESS_WIDE: 0.96 would shrink them too much. */
export const PRESS = { scale: 0.96 }
export const PRESS_WIDE = { scale: 0.985 }
export const PRESS_TRANSITION: Transition = { type: "spring", duration: 0.2, bounce: 0 }

/** Icons that change meaning (mute, tick, bolt): scale 0.25 to 1, opacity 0 to 1, blur 4px to 0. */
export const ICON_HIDDEN = { opacity: 0, scale: 0.25, filter: "blur(4px)" }
export const ICON_SHOWN = { opacity: 1, scale: 1, filter: "blur(0px)" }
export const ICON_TRANSITION: Transition = { type: "spring", duration: 0.3, bounce: 0 }

/** Text that changes (a minute, a percentage): a small drop-in with a touch of blur. */
export const TEXT_HIDDEN = { opacity: 0, y: 3, filter: "blur(2px)" }
export const TEXT_SHOWN = { opacity: 1, y: 0, filter: "blur(0px)" }
export const TEXT_EXIT = { opacity: 0, y: -3, filter: "blur(2px)" }
export const TEXT_TRANSITION: Transition = { duration: 0.18, ease: EASE_OUT }

/** Things that arrive and leave (an app row, a status item): fade, small scale, blur. Leaving is quicker. */
export const ITEM_HIDDEN = { opacity: 0, scale: 0.94, filter: "blur(3px)" }
export const ITEM_SHOWN = { opacity: 1, scale: 1, filter: "blur(0px)" }
export const ITEM_ENTER: Transition = { duration: 0.2, ease: EASE_OUT }
export const ITEM_EXIT: Transition = { duration: 0.12, ease: EASE_OUT }

/** A neighbour sliding into the gap someone left: a no-bounce spring, interruptible. */
export const SLIDE: Transition = { type: "spring", duration: 0.3, bounce: 0 }

/** Honour "reduce motion": movement and scaling stop, fades and colour stay (they carry meaning). */
export function Motion({ children }: { children: ReactNode }) {
  return <MotionConfig reducedMotion="user">{children}</MotionConfig>
}

/**
 * Shows `children` and, when `id` changes, swaps them with the old ones: icons fade + scale + blur,
 * text drops in. The old one leaves while the new one arrives (`popLayout`), the first render does
 * not animate. Keep a static cue next to it (the value itself is always the cue).
 */
export function Swap({
  id,
  kind = "icon",
  className,
  children,
}: {
  id: string | number | boolean
  kind?: "icon" | "text"
  className?: string
  children: ReactNode
}) {
  const icon = kind === "icon"
  return (
    <AnimatePresence initial={false} mode="popLayout">
      <motion.span
        key={String(id)}
        initial={icon ? ICON_HIDDEN : TEXT_HIDDEN}
        animate={icon ? ICON_SHOWN : TEXT_SHOWN}
        exit={icon ? ICON_HIDDEN : TEXT_EXIT}
        transition={icon ? ICON_TRANSITION : TEXT_TRANSITION}
        className={cn("inline-flex", className)}
      >
        {children}
      </motion.span>
    </AnimatePresence>
  )
}

/**
 * Switching from one open panel to another (click Wi-Fi while the control center is open): the new
 * panel fades in and grows from 0.97, from the side the old panel was on. Only opacity and scale, and
 * the scale origin stays inside the panel: the window is exactly as big as the panel, so anything that
 * moves or blurs past its edge would be cut off in a hard line. Opening a panel from nothing stays
 * instant (ADR 0018); only a swap, where the window is already on screen, moves.
 */
export const PANEL_SWAP_FROM = { opacity: 0, scale: 0.97 }
export const PANEL_SWAP_TRANSITION: Transition = { duration: 0.22, ease: EASE_OUT }
