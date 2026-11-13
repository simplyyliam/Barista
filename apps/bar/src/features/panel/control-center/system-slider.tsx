import { useEffect, useState, type ComponentProps } from "react"
import { AnimatePresence, motion } from "motion/react"
import { Slider } from "@barista/ui/components/ui/slider"
import { Icon } from "@barista/ui/icons"

/** The slider moves in steps of this many percent, so a value can be hit exactly. */
const STEP = 5

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

/**
 * The slider of App.png: a thin track, and a thick white head (the filled part) with its icon at
 * the end. While you drag, the icon turns into the value (in percent) and back when you let go.
 */
export function SystemSlider({ label, icon, value, disabled, onChange, onCommit, onIconClick }: SystemSliderProps) {
  const percent = Math.round(Math.min(1, Math.max(0, value)) * 100)
  const first = (v: number | readonly number[]) => (typeof v === "number" ? v : (v[0] ?? 0))
  const [dragging, setDragging] = useState(false)

  // A drag can end outside the slider (the pointer leaves the window): never stay in "dragging".
  useEffect(() => {
    if (!dragging) return
    const stop = () => setDragging(false)
    window.addEventListener("pointerup", stop)
    window.addEventListener("pointercancel", stop)
    return () => {
      window.removeEventListener("pointerup", stop)
      window.removeEventListener("pointercancel", stop)
    }
  }, [dragging])

  return (
    <div className="relative h-cc-slider" onPointerDown={() => !disabled && setDragging(true)}>
      <Slider
        aria-label={label}
        value={[percent]}
        min={0}
        max={100}
        step={STEP}
        disabled={disabled}
        onValueChange={(v) => onChange(first(v) / 100)}
        onValueCommitted={(v) => {
          setDragging(false)
          onCommit(first(v) / 100)
        }}
        className={
          "h-full cursor-default [&>*]:h-full " +
          "[&_[data-slot=slider-track]]:h-1 [&_[data-slot=slider-track]]:overflow-visible [&_[data-slot=slider-track]]:rounded-full [&_[data-slot=slider-track]]:bg-cc-fg/25 " +
          "[&_[data-slot=slider-range]]:absolute! [&_[data-slot=slider-range]]:top-1/2 [&_[data-slot=slider-range]]:h-cc-slider! [&_[data-slot=slider-range]]:min-w-cc-slider [&_[data-slot=slider-range]]:-translate-y-1/2 [&_[data-slot=slider-range]]:rounded-full [&_[data-slot=slider-range]]:bg-cc-fg " +
          "[&_[data-slot=slider-thumb]]:opacity-0"
        }
      />
      {/* Rides on the end of the head (24 px of room, so it never leaves the pill). */}
      <button
        type="button"
        tabIndex={-1}
        aria-label={`${label} icon`}
        disabled={disabled || !onIconClick || dragging}
        onClick={onIconClick}
        style={{ left: `max(0px, calc(${percent}% - 24px))` }}
        className="absolute top-0 flex size-cc-slider items-center justify-center rounded-full text-cc-active outline-none disabled:pointer-events-none"
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
              <Icon icon={icon} size={14} />
            </motion.span>
          )}
        </AnimatePresence>
      </button>
    </div>
  )
}
