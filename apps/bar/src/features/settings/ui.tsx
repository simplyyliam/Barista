import { useId, useState, type KeyboardEvent, type ReactNode } from "react"
import { motion } from "motion/react"
import { Button } from "@barista/ui/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@barista/ui/components/ui/select"
import { Slider } from "@barista/ui/components/ui/slider"
import { Switch } from "@barista/ui/components/ui/switch"
import { cn } from "cn"
import { SLIDE } from "@/shared/motion"

/**
 * A titled group of rows. The title is the same small muted label as the sidebar's group names, and
 * the rows sit on the page with no lines between them.
 */
export function Group({
  title,
  description,
  onReset,
  children,
}: {
  title: string
  description?: string
  /** Puts the group back to its defaults. */
  onReset?: () => void
  children: ReactNode
}) {
  return (
    <section aria-label={title} className="flex flex-col">
      <div className="flex min-h-6 items-center justify-between pb-1">
        <h3 className="text-[11px] text-muted-foreground/70">
          {title}
          {description && <span className="ml-2 text-muted-foreground/50">{description}</span>}
        </h3>
        {onReset && (
          <button
            type="button"
            onClick={onReset}
            className="rounded px-1 text-[11px] text-muted-foreground/70 outline-none transition-colors hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/40"
          >
            Reset
          </button>
        )}
      </div>
      <div className="flex flex-col">{children}</div>
    </section>
  )
}

/** One setting: its name and a one-line hint on the left, the control on the right. */
export function Row({
  label,
  hint,
  children,
  disabled,
}: {
  label: string
  hint?: string
  children: ReactNode
  disabled?: boolean
}) {
  return (
    <div className={cn("flex min-h-10 items-center justify-between gap-6 py-2", disabled && "opacity-40")}>
      <div className="flex min-w-0 flex-col gap-0.5">
        <div className="text-[13px] text-foreground">{label}</div>
        {hint && <div className="text-xs leading-snug text-muted-foreground/70">{hint}</div>}
      </div>
      <div className="flex shrink-0 items-center gap-3">{children}</div>
    </div>
  )
}

export function SwitchRow({
  label,
  hint,
  checked,
  onChange,
  disabled,
}: {
  label: string
  hint?: string
  checked: boolean
  onChange: (checked: boolean) => void
  disabled?: boolean
}) {
  return (
    <Row label={label} hint={hint} disabled={disabled}>
      <Switch checked={checked} disabled={disabled} onCheckedChange={onChange} aria-label={label} />
    </Row>
  )
}

/**
 * Two to four choices side by side. A small dark track with a lighter pill that slides to the chosen
 * one. More than that is a `SelectRow`. Arrow keys move the choice (a radio group).
 */
export function Segmented<T extends string>({
  value,
  options,
  onChange,
  label,
  disabled,
}: {
  value: T
  options: { value: T; label: string }[]
  onChange: (value: T) => void
  label: string
  disabled?: boolean
}) {
  const id = useId()
  const move = (e: KeyboardEvent, by: number) => {
    e.preventDefault()
    const at = options.findIndex((o) => o.value === value)
    const next = options[(at + by + options.length) % options.length]
    if (next) onChange(next.value)
  }
  return (
    <div
      role="radiogroup"
      aria-label={label}
      aria-disabled={disabled}
      className={cn("flex h-7 rounded-lg bg-white/[0.06] p-0.5", disabled && "pointer-events-none opacity-50")}
    >
      {options.map((option) => {
        const on = option.value === value
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={on}
            tabIndex={on ? 0 : -1}
            onClick={() => onChange(option.value)}
            onKeyDown={(e) => {
              if (e.key === "ArrowRight" || e.key === "ArrowDown") move(e, 1)
              if (e.key === "ArrowLeft" || e.key === "ArrowUp") move(e, -1)
            }}
            className={cn(
              "relative rounded-md px-3 text-xs whitespace-nowrap outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/40",
              on ? "text-foreground" : "text-muted-foreground hover:text-foreground/80",
            )}
          >
            {on && (
              <motion.span
                layoutId={`seg-${id}`}
                transition={SLIDE}
                className="absolute inset-0 rounded-md bg-white/[0.12]"
              />
            )}
            <span className="relative">{option.label}</span>
          </button>
        )
      })}
    </div>
  )
}

export function SegmentedRow<T extends string>({
  label,
  hint,
  value,
  options,
  onChange,
  disabled,
}: {
  label: string
  hint?: string
  value: T
  options: { value: T; label: string }[]
  onChange: (value: T) => void
  disabled?: boolean
}) {
  return (
    <Row label={label} hint={hint} disabled={disabled}>
      <Segmented label={label} value={value} options={options} onChange={onChange} disabled={disabled} />
    </Row>
  )
}

export function SliderRow({
  label,
  hint,
  value,
  min,
  max,
  step = 1,
  unit = "",
  onChange,
  disabled,
}: {
  label: string
  hint?: string
  value: number
  min: number
  max: number
  step?: number
  unit?: string
  onChange: (value: number) => void
  disabled?: boolean
}) {
  // Follows the thumb while it moves, and tells Rust once when it is let go (every change is a
  // write to the settings file, so a drag must not send a hundred).
  const [draft, setDraft] = useState<number | null>(null)
  const shown = draft ?? value
  const first = (v: number | readonly number[]) => (Array.isArray(v) ? (v[0] ?? value) : (v as number))
  return (
    <Row label={label} hint={hint} disabled={disabled}>
      {/* The slider fills its parent (`w-full` is built in), so the parent sets the length. */}
      <div className="w-44">
        <Slider
          aria-label={label}
          // A hairline track with a small white thumb. `!` because the shared slider sets its own sizes.
          className="[&_[data-slot=slider-range]]:bg-white/85 [&_[data-slot=slider-thumb]]:size-3.5! [&_[data-slot=slider-thumb]]:rounded-full! [&_[data-slot=slider-thumb]]:border-0! [&_[data-slot=slider-thumb]]:shadow-[0_1px_4px_rgba(0,0,0,0.6)] [&_[data-slot=slider-track]]:h-1! [&_[data-slot=slider-track]]:rounded-full! [&_[data-slot=slider-track]]:bg-white/10!"
          min={min}
          max={max}
          step={step}
          value={[shown]}
          disabled={disabled}
          onValueChange={(v) => setDraft(first(v))}
          onValueCommitted={(v) => {
            setDraft(null)
            onChange(first(v))
          }}
        />
      </div>
      <span className="w-12 text-right text-xs whitespace-nowrap tabular-nums text-muted-foreground">
        {shown}
        {unit}
      </span>
    </Row>
  )
}

/** For five or more choices, or a list that can grow (monitors). A shadcn select. */
export function SelectRow({
  label,
  hint,
  value,
  options,
  onChange,
  disabled,
}: {
  label: string
  hint?: string
  value: string
  options: { value: string; label: string }[]
  onChange: (value: string) => void
  disabled?: boolean
}) {
  return (
    <Row label={label} hint={hint} disabled={disabled}>
      <Select items={options} value={value} disabled={disabled} onValueChange={(next) => next && onChange(String(next))}>
        <SelectTrigger aria-label={label} className="h-7 w-48 border-0 bg-white/[0.06] text-xs">
          <SelectValue />
        </SelectTrigger>
        {/* Opens below the trigger, not over it like a Mac select. */}
        <SelectContent alignItemWithTrigger={false} align="end">
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </Row>
  )
}

/** Same size and weight as a sidebar item, and level with the first one. */
export function PageTitle({ title, description }: { title: string; description?: string }) {
  return (
    <header className="flex flex-col pb-3">
      <h2 className="flex h-8 items-center text-[13px] font-medium text-foreground">{title}</h2>
      {description && <p className="max-w-md text-xs text-muted-foreground/70">{description}</p>}
    </header>
  )
}

/** A small button for the settings pages (shadcn Button). */
export function ActionButton({
  children,
  onClick,
  tone = "default",
  disabled,
}: {
  children: ReactNode
  onClick: () => void
  tone?: "default" | "danger"
  disabled?: boolean
}) {
  return (
    <Button
      variant={tone === "danger" ? "destructive" : "secondary"}
      size="sm"
      className="h-7 border-0 bg-white/[0.06] text-xs hover:bg-white/[0.1]" disabled={disabled} onClick={onClick}>
      {children}
    </Button>
  )
}
