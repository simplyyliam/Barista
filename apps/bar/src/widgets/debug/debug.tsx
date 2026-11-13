import { useEffect, useState } from "react"
import { commands } from "@/shared/bindings"
import { BarPreview } from "@/features/debug/bar-preview"
import { Controls } from "@/features/debug/controls"
import { DEFAULT_DEBUG_STATE, type DebugState } from "@/features/debug/debug-state"

// A fixed time, so the clock never changes under the eye while testing.
const NOW = new Date(2026, 10, 12, 13, 18)

/**
 * The debug window (debug builds, Ctrl+Alt+Shift+D). Shows the bar on a light and a dark strip
 * with made-up battery, Wi-Fi, volume and clock states. Never touches the real services.
 */
export default function DebugWindow() {
  const [state, setState] = useState<DebugState>(DEFAULT_DEBUG_STATE)
  const [version, setVersion] = useState("…")
  const patch = (change: Partial<DebugState>) => setState((s) => ({ ...s, ...change }))

  useEffect(() => {
    // Proves the Rust <-> TypeScript bridge works.
    void commands.getVersion().then(setVersion)
  }, [])

  return (
    <div className="h-full overflow-auto bg-background p-6 font-sans text-foreground">
      <header className="mb-4 flex items-baseline justify-between">
        <h1 className="text-lg font-semibold">Barista debug</h1>
        <span className="text-sm text-muted-foreground">version {version}</span>
      </header>
      <div className="mb-6 flex flex-col gap-3">
        <BarPreview tone="light" state={state} now={NOW} />
        <BarPreview tone="dark" state={state} now={NOW} />
      </div>
      <Controls state={state} patch={patch} />
    </div>
  )
}
