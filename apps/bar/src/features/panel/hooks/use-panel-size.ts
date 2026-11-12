import { useLayoutEffect, type RefObject } from "react"
import { commands } from "@/shared/bindings"

/**
 * Tells Rust how big the content is (logical px) so it can place the window under its item and
 * show it. The first report goes out before the browser paints; later ones only on a real change.
 * Pass a new `key` for every open so the first report is sent again, and `null` while closed.
 */
export function usePanelSize(ref: RefObject<HTMLElement | null>, key: string | null) {
  useLayoutEffect(() => {
    const element = ref.current
    // `null` = the panel is closed (its content stays mounted, hidden, so it is ready for the next open).
    if (!element || key === null) return

    let last = ""
    const send = () => {
      // offsetWidth/offsetHeight, not getBoundingClientRect: the opening motion scales the element
      // (96% at the first frame), and that must not make the window 4% too small.
      const width = element.offsetWidth
      const height = element.offsetHeight
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
