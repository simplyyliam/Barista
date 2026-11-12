import { useEffect, useState } from "react"

/** Keeps one snapshot from Rust fresh: listen first, then ask once, replace on every event. */
export function useSnapshot<T>(
  get: () => Promise<T>,
  listen: (handler: (value: T) => void) => Promise<() => void>,
): T | null {
  const [value, setValue] = useState<T | null>(null)

  useEffect(() => {
    let cancelled = false
    let gotEvent = false
    let stop: (() => void) | undefined
    void (async () => {
      const unlisten = await listen((next) => {
        gotEvent = true
        setValue(next)
      })
      if (cancelled) return unlisten()
      stop = unlisten
      const current = await get()
      if (!cancelled && !gotEvent) setValue(current)
    })()
    return () => {
      cancelled = true
      stop?.()
    }
    // get and listen are module-level functions: they never change.
  }, [])

  return value
}
