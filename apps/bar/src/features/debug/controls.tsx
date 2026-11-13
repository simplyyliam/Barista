import type { ReactNode } from "react"
import { Slider } from "@barista/ui/components/ui/slider"
import { Switch } from "@barista/ui/components/ui/switch"
import type { DebugState } from "./debug-state"

type Patch = (change: Partial<DebugState>) => void

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex min-h-7 items-center justify-between gap-4 text-sm">
      <span>{label}</span>
      <span className="flex w-44 items-center justify-end gap-2">{children}</span>
    </div>
  )
}

function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (on: boolean) => void }) {
  return (
    <Row label={label}>
      <Switch checked={checked} onCheckedChange={onChange} />
    </Row>
  )
}

function Range({
  label,
  value,
  max,
  step,
  show,
  onChange,
}: {
  label: string
  value: number
  max: number
  step: number
  show: string
  onChange: (value: number) => void
}) {
  return (
    <Row label={label}>
      <Slider
        value={[value]}
        min={0}
        max={max}
        step={step}
        onValueChange={(next) => onChange(Array.isArray(next) ? next[0] : next)}
      />
      <span className="w-10 text-right tabular-nums">{show}</span>
    </Row>
  )
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-1">
      <h2 className="mb-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{title}</h2>
      {children}
    </section>
  )
}

/** Sliders and switches for the bar states that come from our own items: Wi-Fi, volume, clock, pills, height. */
export function Controls({ state, patch }: { state: DebugState; patch: Patch }) {
  return (
    <div className="grid grid-cols-2 gap-x-10 gap-y-6">
      <Group title="Wi-Fi and volume">
        <Range label="Wi-Fi arcs" value={state.wifiStrength} max={3} step={1} show={String(state.wifiStrength)} onChange={(v) => patch({ wifiStrength: v as DebugState["wifiStrength"] })} />
        <Toggle label="Wi-Fi off" checked={state.wifiOff} onChange={(v) => patch({ wifiOff: v })} />
        <Range label="Volume" value={Math.round(state.volume * 100)} max={100} step={1} show={`${Math.round(state.volume * 100)}%`} onChange={(v) => patch({ volume: v / 100 })} />
        <Toggle label="Muted" checked={state.muted} onChange={(v) => patch({ muted: v })} />
      </Group>
      <Group title="Clock">
        <Toggle label="24 hour clock" checked={state.hour24} onChange={(v) => patch({ hour24: v })} />
      </Group>
      <Group title="Pills">
        <Toggle label="Pills open" checked={state.allOpen} onChange={(v) => patch({ allOpen: v })} />
        <Range label="Bar height" value={state.height} max={40} step={1} show={`${state.height}px`} onChange={(v) => patch({ height: Math.max(20, v) })} />
      </Group>
    </div>
  )
}
