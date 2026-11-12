import { useState } from "react"
import { commands } from "@/shared/bindings"
import { Icon, LucideMinus, LucidePlus } from "@barista/ui/icons"

const STEP = 5
const MIN = 5
const MAX = 240

const pill =
  "flex h-[43px] items-center rounded-full bg-panel text-[13px] text-panel-fg outline-none"
const ring = "outline-none focus-visible:ring-2 focus-visible:ring-panel-fg/50 rounded-full"

/**
 * The two pills under the calendar. Windows has no way to start Focus for an app, so "Focus" opens
 * Windows' own Focus page; a Barista focus timer comes in a later release.
 */
export function FocusRow() {
  const [minutes, setMinutes] = useState(30)
  const change = (delta: number) => setMinutes((m) => Math.min(MAX, Math.max(MIN, m + delta)))

  return (
    <div className="flex items-center justify-center gap-2.5">
      <div className={`${pill} gap-3 px-4`}>
        <button type="button" aria-label="Longer" onClick={() => change(STEP)} className={ring}>
          <Icon icon={LucidePlus} size={14} />
        </button>
        <span className="min-w-14 text-center tabular-nums">{minutes} mins</span>
        <button type="button" aria-label="Shorter" onClick={() => change(-STEP)} className={ring}>
          <Icon icon={LucideMinus} size={14} />
        </button>
      </div>
      <button
        type="button"
        onClick={() => void commands.openFocusSettings()}
        className={`${pill} px-5 focus-visible:ring-2 focus-visible:ring-panel-fg/50`}
      >
        Focus
      </button>
    </div>
  )
}
