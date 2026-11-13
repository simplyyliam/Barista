import { useEffect, useRef, useState, type ComponentProps, type KeyboardEvent, type PointerEvent } from "react"
import { AnimatePresence, motion } from "motion/react"
import { Icon } from "@barista/ui/icons"
import { cn } from "cn"
import { clamp01, toPercentStep, valueForKey, valueFromPointer } from "./lib/slider-math"

type SystemSliderProps = {
  label: string
  icon: ComponentProps<typeof Icon>["icon"]
  /** 0 to 1. */
  value: number
  disabled?: boolean
  onChange: (value: number) => void
  onCommit: (value: number) => void
  /** Click on the icon (volume: mute). */
  onIconClick?: () => void
}

// Fade, shrink and blur in one go; the icon and the number swap places with this.
const HIDDEN = { opacity: 0, scale: 0.5, filter: "blur(4px)" }
const SHOWN = { opacity: 1, scale: 1, filter: "blur(0px)" }
const SWAP = { duration: 0.16, ease: [0.2, 0, 0, 1] as const }

/** The icon is 14px with 8px on both sides inside the head, so the head is never narrower than this. */
const ICON = 14
const ICON_PAD = 8
const HEAD_MIN = ICON + ICON_PAD * 2
/** A press on the icon is a click until the pointer has moved this far, then it is a drag. */
const DRAG_START = 4

/**
 * The slider of App.png: a thin track, and a thick white head (the filled part) with its icon at
 * the end. While you drag, the icon turns into the value (in percent) and back when you let go.
 *
 * Pointer handling is our own, not a slider library's: the head's right edge sits exactly under
 * the pointer (the library inset its thumb, so the bar and the cursor disagreed), the pointer is
 * captured so a drag can leave the bar, and the value is continuous (whole percent goes to Rust).
 */
export function SystemSlider({ label, icon, value, disabled, onChange, onCommit, onIconClick }: SystemSliderProps) {
  const root = useRef<HTMLDivElement>(null)
  const [live, setLive] = useState<number | null>(null)
  // After you let go, the value from Rust is still the old one for a moment. Keep showing what you
  // set until it arrives (or half a second passes), so the head never snaps back.
  const [held, setHeld] = useState<number | null>(null)
  const liveRef = useRef<number | null>(null)
  const press = useRef<{ startX: number; onIcon: boolean; moved: boolean } | null>(null)

  const dragging = live !== null
  const shown = clamp01(live ?? held ?? value)
  const percent = Math.round(shown * 100)

  const fromPointer = (clientX: number) => {
    const rect = root.current?.getBoundingClientRect()
    return rect ? valueFromPointer(clientX, rect.left, rect.width) : 0
  }

  const move = (clientX: number) => {
    const next = fromPointer(clientX)
    liveRef.current = next
    setLive(next)
    onChange(toPercentStep(next))
  }

  const finish = (commit: boolean) => {
    const last = liveRef.current
    liveRef.current = null
    setLive(null)
    if (commit && last !== null) {
      setHeld(last)
      onCommit(toPercentStep(last))
    }
  }

  useEffect(() => {
    if (held === null) return
    if (Math.abs(value - held) <= 0.011) {
      setHeld(null)
      return
    }
    const timer = setTimeout(() => setHeld(null), 500)
    return () => clearTimeout(timer)
  }, [held, value])

  /** Is the pointer on the icon at the head's end (with a little room around it)? */
  const onIconAt = (clientX: number) => {
    const rect = root.current?.getBoundingClientRect()
    if (!rect || !onIconClick) return false
    const headEnd = rect.left + Math.max(HEAD_MIN, shown * rect.width)
    return clientX >= headEnd - ICON_PAD - ICON - 4 && clientX <= headEnd
  }

  const onPointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (disabled || event.button !== 0) return
    event.currentTarget.setPointerCapture(event.pointerId)
    const onIcon = onIconAt(event.clientX)
    press.current = { startX: event.clientX, onIcon, moved: false }
    // A press on the icon waits to see if it is a click (mute) or the start of a drag.
    if (!onIcon) move(event.clientX)
  }

  const onPointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const p = press.current
    if (!p) return
    if (p.onIcon && !p.moved) {
      if (Math.abs(event.clientX - p.startX) < DRAG_START) return
      p.moved = true
    }
    move(event.clientX)
  }

  const onPointerUp = (event: PointerEvent<HTMLDivElement>) => {
    const p = press.current
    press.current = null
    if (!p) return
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId)
    if (p.onIcon && !p.moved) return onIconClick?.()
    finish(true)
  }

  const onPointerCancel = () => {
    press.current = null
    finish(true)
  }

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (disabled) return
    const next = valueForKey(event.key, shown)
    if (next === null) return
    event.preventDefault()
    onChange(next)
    onCommit(next)
  }

  return (
    <div
      ref={root}
      role="slider"
      tabIndex={disabled ? -1 : 0}
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent}
      aria-valuetext={`${percent}%`}
      aria-disabled={disabled}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerCancel}
      onKeyDown={onKeyDown}
      className={cn(
        "relative h-cc-slider touch-none rounded-full outline-none select-none focus-visible:ring-2 focus-visible:ring-cc-fg/60",
        disabled && "opacity-50",
      )}
    >
      <div className="absolute inset-x-0 top-1/2 h-1 -translate-y-1/2 rounded-full bg-cc-fg/25" />
      <div
        // Follows the pointer exactly while dragging; eases to a new value (a volume key, another
        // app) when not.
        style={{ width: `max(${HEAD_MIN}px, ${(shown * 100).toFixed(3)}%)` }}
        className={cn(
          "absolute top-0 left-0 h-full rounded-full bg-cc-fg",
          !dragging && "transition-[width] duration-150 ease-out motion-reduce:transition-none",
        )}
      >
        {/* The icon (or the number while dragging) sits ICON_PAD from the head's end. */}
        <div
          style={{ right: ICON_PAD, width: ICON }}
          className="pointer-events-none absolute inset-y-0 flex items-center justify-center text-cc-active"
        >
          <AnimatePresence initial={false} mode="popLayout">
            {dragging ? (
              <motion.span
                key="value"
                initial={HIDDEN}
                animate={SHOWN}
                exit={HIDDEN}
                transition={SWAP}
                className="text-[11px] leading-none font-medium tabular-nums"
              >
                {percent}
              </motion.span>
            ) : (
              <motion.span key="icon" initial={HIDDEN} animate={SHOWN} exit={HIDDEN} transition={SWAP} className="flex">
                <Icon icon={icon} size={ICON} />
              </motion.span>
            )}
          </AnimatePresence>
        </div>
      </div>
    </div>
  )
}
