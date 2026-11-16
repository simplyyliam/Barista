import { useState, type ReactNode } from "react"
import { Button } from "@barista/ui/components/ui/button"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@barista/ui/components/ui/select"
import { Slider } from "@barista/ui/components/ui/slider"
import { Switch } from "@barista/ui/components/ui/switch"
import { ToggleGroup, ToggleGroupItem } from "@barista/ui/components/ui/toggle-group"
import { cn } from "cn"

/**
 * A titled group of rows on a page. No box around it: the rows sit on the page itself and a hairline
 * tells them apart, so the page reads as one calm column.
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
      <div className="flex items-end justify-between pb-2">
        <div className="flex flex-col gap-0.5">
          <h3 className="text-[15px] font-medium tracking-tight text-foreground">{title}</h3>
          {description && <p className="text-[13px] text-muted-foreground">{description}</p>}
        </div>
        {onReset && (
          <Button variant="ghost" size="sm" onClick={onReset} className="text-muted-foreground">
            Reset
          </Button>
        )}
      </div>
      <div className="flex flex-col divide-y divide-white/[0.07] border-t border-white/[0.07]">{children}</div>
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
    <div className={cn("flex min-h-16 items-center justify-between gap-8 py-3.5", disabled && "opacity-40")}>
      <div className="flex min-w-0 flex-col gap-0.5">
        <div className="text-sm text-foreground">{label}</div>
        {hint && <div className="text-[13px] leading-snug text-muted-foreground">{hint}</div>}
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

/** Two to four choices side by side (a shadcn toggle group). More than that is a `SelectRow`. */
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
  return (
    <ToggleGroup
      aria-label={label}
      variant="outline"
      size="sm"
      spacing={0}
      disabled={disabled}
      value={[value]}
      // A group with one choice always keeps one: ignore the click that would clear it.
      onValueChange={(next) => next[0] && onChange(next[0] as T)}
    >
      {options.map((option) => (
        <ToggleGroupItem
          key={option.value}
          value={option.value}
          className="px-3.5 data-pressed:bg-white data-pressed:text-black"
        >
          {option.label}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
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
      <div className="w-80">
        <Slider
          aria-label={label}
          // A thick track and a big round thumb you can grab. `!` because the shared slider sets its own sizes.
          className="[&_[data-slot=slider-thumb]]:size-6! [&_[data-slot=slider-thumb]]:rounded-full! [&_[data-slot=slider-thumb]]:border-0! [&_[data-slot=slider-thumb]]:shadow-lg [&_[data-slot=slider-track]]:h-2.5! [&_[data-slot=slider-track]]:rounded-full!"
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
      <span className="w-16 text-right text-sm whitespace-nowrap tabular-nums text-muted-foreground">
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
        <SelectTrigger aria-label={label} className="w-60">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
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

export function PageTitle({ title, description }: { title: string; description?: string }) {
  return (
    <header className="flex flex-col gap-1.5 pb-2">
      <h2 className="text-2xl font-medium tracking-tight text-foreground">{title}</h2>
      {description && <p className="max-w-xl text-sm text-muted-foreground">{description}</p>}
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
    <Button variant={tone === "danger" ? "destructive" : "secondary"} size="sm" disabled={disabled} onClick={onClick}>
      {children}
    </Button>
  )
}
