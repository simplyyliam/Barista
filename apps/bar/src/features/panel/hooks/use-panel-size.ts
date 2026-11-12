import { useLayoutEffect, type RefObject } from "react"
import { commands } from "@/shared/bindings"

/**
 * Tells Rust how big the content is (logical px) so it can place the window under its item and
 * show it. The first report goes out before the browser paints; later ones only on a real change.
 * Pass a new `key` for every open so the first report is sent again.
 */
export function usePanelSize(ref: RefObject<HTMLElement | null>, key: string) {
  useLayoutEffect(() => {
    const element = ref.current
    if (!element) return

    let last = ""
    const send = () => {
      const rect = element.getBoundingClientRect()
      const width = Math.ceil(rect.width)
      const height = Math.ceil(rect.height)
      const size = `${width}x${height}`
      if (size === last) return
      last = size
      void commands.panelResized(width, height)
    }

    send()
    const observer = new ResizeObserver(send)
    observer.observe(element)
    return () => observer.disconnect()
  }, [ref, key])
}
