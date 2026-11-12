import { useEffect, useState } from "react"
import { events } from "@/shared/bindings"

/**
 * True for a moment at startup, while Rust shows the panel window invisibly to warm it up. The
 * panel draws its content during that moment (the pictures are costly to paint the first time), so
 * the first real open is as fast as the next ones.
 */
export function useWarm() {
  const [warm, setWarm] = useState(false)

  useEffect(() => {
    let stop: (() => void) | undefined
    let cancelled = false
    void events.panelWarm.listen((event) => setWarm(event.payload)).then((off) => {
      if (cancelled) off()
      else stop = off
    })
    return () => {
      cancelled = true
      stop?.()
    }
  }, [])

  return warm
}
