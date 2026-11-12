import type { ComponentProps } from "react"
import { Slider } from "@barista/ui/components/ui/slider"
import { Icon } from "@barista/ui/icons"

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

/** The slider of App.png: a thin track, and a thick white head (the filled part) with its icon at the end. */
export function SystemSlider({ label, icon, value, disabled, onChange, onCommit, onIconClick }: SystemSliderProps) {
  const percent = Math.round(Math.min(1, Math.max(0, value)) * 100)
  const first = (v: number | readonly number[]) => (typeof v === "number" ? v : (v[0] ?? 0))

  return (
    <div className="relative h-cc-slider">
      <Slider
        aria-label={label}
        value={[percent]}
        min={0}
        max={100}
        disabled={disabled}
        onValueChange={(v) => onChange(first(v) / 100)}
        onValueCommitted={(v) => onCommit(first(v) / 100)}
        className={
          "h-full cursor-default [&>*]:h-full " +
          "[&_[data-slot=slider-track]]:h-1 [&_[data-slot=slider-track]]:overflow-visible [&_[data-slot=slider-track]]:rounded-full [&_[data-slot=slider-track]]:bg-cc-fg/25 " +
          "[&_[data-slot=slider-range]]:absolute! [&_[data-slot=slider-range]]:top-1/2 [&_[data-slot=slider-range]]:h-cc-slider! [&_[data-slot=slider-range]]:min-w-cc-slider [&_[data-slot=slider-range]]:-translate-y-1/2 [&_[data-slot=slider-range]]:rounded-full [&_[data-slot=slider-range]]:bg-cc-fg " +
          "[&_[data-slot=slider-thumb]]:opacity-0"
        }
      />
      {/* The icon rides on the end of the fill (24 px of room, so it never leaves the pill). */}
      <button
        type="button"
        tabIndex={-1}
        aria-label={`${label} icon`}
        disabled={disabled || !onIconClick}
        onClick={onIconClick}
        style={{ left: `max(0px, calc(${percent}% - 24px))` }}
        className="absolute top-0 flex size-cc-slider items-center justify-center rounded-full text-cc-active outline-none disabled:pointer-events-none"
      >
        <Icon icon={icon} size={14} />
      </button>
    </div>
  )
}
