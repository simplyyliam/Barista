import { useState, type ReactNode } from "react"
import { Slider } from "@barista/ui/components/ui/slider"
import { Switch } from "@barista/ui/components/ui/switch"
import { cn } from "cn"

/** A titled group of rows on a page, like one card in macOS System Settings. */
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
    <section aria-label={title} className="flex flex-col gap-2">
      <div className="flex items-baseline justify-between px-1">
        <div>
          <h3 className="text-[13px] font-medium text-foreground">{title}</h3>
          {description && <p className="text-xs text-muted-foreground">{description}</p>}
        </div>
        {onReset && (
          <button
            type="button"
            onClick={onReset}
            className="rounded px-1 text-xs text-muted-foreground outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/40"
          >
            Reset
          </button>
        )}
      </div>
      <div className="divide-y divide-border/60 rounded-xl bg-card/60 ring-1 ring-border/60">{children}</div>
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
    <div className={cn("flex min-h-11 items-center justify-between gap-4 px-3.5 py-2", disabled && "opacity-45")}>
      <div className="min-w-0">
        <div className="text-[13px] text-foreground">{label}</div>
        {hint && <div className="text-xs text-muted-foreground">{hint}</div>}
      </div>
      <div className="flex shrink-0 items-center gap-2">{children}</div>
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

/** Two to four choices side by side. More than that is a `SelectRow`. */
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
    <div role="radiogroup" aria-label={label} className="flex rounded-lg bg-muted p-0.5">
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          disabled={disabled}
          onClick={() => onChange(option.value)}
          className={cn(
            "rounded-md px-2.5 py-1 text-xs outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/40",
            option.value === value
              ? "bg-background text-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          {option.label}
        </button>
      ))}
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
      <Slider
        aria-label={label}
        className="w-40"
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
      <span className="w-12 text-right text-xs tabular-nums text-muted-foreground">
        {shown}
        {unit}
      </span>
    </Row>
  )
}

/** For five or more choices, or a list that can grow (monitors). */
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
      <select
        aria-label={label}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        className="h-7 max-w-56 rounded-md bg-muted px-2 text-xs text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring/40"
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </Row>
  )
}

export function PageTitle({ title, description }: { title: string; description?: string }) {
  return (
    <header className="flex flex-col gap-1">
      <h2 className="text-[15px] font-medium text-foreground">{title}</h2>
      {description && <p className="text-[13px] text-muted-foreground">{description}</p>}
    </header>
  )
}

/** A small button for the settings pages. */
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
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={cn(
        "h-7 rounded-md px-2.5 text-xs outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring/40 disabled:opacity-50",
        tone === "danger" ? "bg-destructive/15 text-destructive hover:bg-destructive/25" : "bg-muted text-foreground hover:bg-muted/70",
      )}
    >
      {children}
    </button>
  )
}
