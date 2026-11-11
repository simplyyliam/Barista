import { useLayoutEffect, type RefObject } from "react"
import { useLatest } from "./use-latest"

/**
 * Tells Rust how big the panel is (logical px) so it can size and place the window.
 * The first report is sent before the browser paints, even in a hidden window, because Rust
 * only shows the window after it. Later ones only when the size really changed.
 */
export function useMenuSize(
  ref: RefObject<HTMLElement | null>,
  report: (width: number, height: number) => void,
) {
  const reportRef = useLatest(report)

  useLayoutEffect(() => {
    const element = ref.current
    if (!element) return

    let last = ""
    const send = (force: boolean) => {
      const rect = element.getBoundingClientRect()
      const width = Math.ceil(rect.width)
      const height = Math.ceil(rect.height)
      const key = `${width}x${height}`
      if (!force && key === last) return
      last = key
      reportRef.current(width, height)
    }

    send(true)
    const observer = new ResizeObserver(() => send(false))
    observer.observe(element)
    return () => observer.disconnect()
  }, [ref, reportRef])
}
