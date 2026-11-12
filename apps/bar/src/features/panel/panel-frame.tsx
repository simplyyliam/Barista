import { useRef, type ReactNode } from "react"
import type { PanelChrome } from "@/shared/bindings"
import { usePanelSize } from "./hooks/use-panel-size"

type PanelFrameProps = {
  /** The window has the system acrylic backdrop; if not, draw an opaque panel. */
  acrylic: boolean
  /** `glass`: the window is the panel. `none`: a bare transparent window, the content draws its own cards. */
  chrome: PanelChrome
  /** New for every open, so the size is reported again. `null` while the panel is closed. */
  openKey: string | null
  children: ReactNode
}

/** The glass of the panel window (or nothing, for floating cards). */
export function PanelFrame({ acrylic, chrome, openKey, children }: PanelFrameProps) {
  const measureRef = useRef<HTMLDivElement>(null)
  usePanelSize(measureRef, openKey)

  if (chrome === "none") {
    return (
      <div
        onContextMenu={(event) => event.preventDefault()}
        className="fixed inset-0 cursor-default overflow-hidden font-bar select-none"
      >
        {/*
          The opening motion. Windows' own window animation does not play on a bare transparent
          window (measured: it appears in one frame), so the cards do it themselves: they wait
          slightly up and small while closed, then fade, slide down and grow into place from the top.
          The content was mounted while hidden, so nothing loads during the motion.
        */}
        <div
          ref={measureRef}
          data-open={openKey !== null}
          className="w-max origin-top data-[open=false]:-translate-y-2 data-[open=false]:scale-[0.96] data-[open=false]:opacity-0 data-[open=true]:transition-[opacity,scale,translate] data-[open=true]:duration-[260ms] data-[open=true]:ease-[cubic-bezier(0.16,1,0.3,1)] motion-reduce:transition-none"
        >
          {children}
        </div>
      </div>
    )
  }

  return (
    <div
      data-acrylic={acrylic}
      onContextMenu={(event) => event.preventDefault()}
      className="fixed inset-0 cursor-default overflow-hidden rounded-[var(--window-radius,8px)] bg-menu-opaque font-bar text-menu-fg shadow-[inset_0_0_0_1px_var(--color-menu-ring)] select-none data-[acrylic=true]:bg-menu-tint"
    >
      <div ref={measureRef} className="w-max min-w-64">
        {children}
      </div>
    </div>
  )
}
