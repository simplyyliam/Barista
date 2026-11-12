import { useEffect, useState } from "react"
import { commands, events, type PanelOpen } from "@/shared/bindings"

/** The panel Rust says is open (null = closed). Subscribes first, then asks, so no event slips by. */
export function useOpenPanel() {
  const [open, setOpen] = useState<PanelOpen | null>(null)

  useEffect(() => {
    let cancelled = false
    let gotEvent = false
    let stop: (() => void) | undefined

    async function start() {
      try {
        const unlisten = await events.panelChanged.listen(({ payload }) => {
          gotEvent = true
          setOpen(payload.open)
        })
        if (cancelled) return unlisten()
        stop = unlisten
        const current = await commands.getOpenPanel()
        if (!cancelled && !gotEvent) setOpen(current)
      } catch (error) {
        console.error("Could not start the panel window:", error)
      }
    }

    void start()
    return () => {
      cancelled = true
      stop?.()
    }
  }, [])

  return open
}
