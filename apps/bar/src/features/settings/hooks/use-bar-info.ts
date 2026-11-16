import { useEffect, useState } from "react"
import { commands, events, type BarInfo } from "@/shared/bindings"

/** The bar's monitor and the list of monitors (`getBarInfo`), kept fresh by `barChanged`. */
export function useBarInfo(): BarInfo | null {
  const [info, setInfo] = useState<BarInfo | null>(null)
  useEffect(() => {
    let cancelled = false
    let gotEvent = false
    let stop: (() => void) | undefined
    void (async () => {
      const unlisten = await events.barChanged.listen((e) => {
        gotEvent = true
        setInfo(e.payload)
      })
      if (cancelled) return unlisten()
      stop = unlisten
      const result = await commands.getBarInfo()
      if (!cancelled && !gotEvent && result.status === "ok") setInfo(result.data)
    })()
    return () => {
      cancelled = true
      stop?.()
    }
  }, [])
  return info
}
