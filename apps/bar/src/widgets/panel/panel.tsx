import { useEffect, useRef } from "react"
import { commands, type PanelOpen } from "@/shared/bindings"
import { PanelBody, PanelFrame, useOpenPanel, useWarm } from "@/features/panel"
import { Motion } from "@/shared/motion"
import { usePanelSwap } from "@/features/panel/hooks/use-panel-swap"

/**
 * The panel window (label `panel`): one window, reused for every system panel. Rust decides when
 * it opens and where it goes; this draws the route named in `panelChanged`, reports its size and
 * asks Rust to close on Esc (ADR 0016).
 *
 * The frame and the body stay mounted while the window is hidden, so the next open has nothing
 * left to load: the data, the icons and the decoded pictures are already there ("prebaked").
 */
export default function PanelWindow() {
  const open = useOpenPanel()
  const warm = useWarm()
  // Closed: remember how the last panel looked, so nothing flips while the window is hidden.
  const last = useRef<PanelOpen | null>(null)
  if (open) last.current = open
  const look = open ?? last.current
  const swap = usePanelSwap(open)

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") void commands.closePanel()
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [])

  return (
    <Motion>
    <PanelFrame
      acrylic={look?.acrylic ?? false}
      chrome={look?.chrome ?? "none"}
      // While Rust warms the hidden window up, report the size as if it were open (the key is only
      // used to start the report; Rust sizes the hidden window from it).
      openKey={open ? `${open.name}|${open.anchor}` : warm ? "warm" : null}
    >
      <div ref={swap}>
        <PanelBody name={open?.name ?? null} warm={warm} />
      </div>
    </PanelFrame>
    </Motion>
  )
}
