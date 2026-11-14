import { useEffect } from "react"

/**
 * Calls `collapse` once the pointer has been away from the bar for `delay` ms. The tray group is
 * opened to reach one icon; left open it would fill the bar for good.
 */
export function useAutoCollapse(active: boolean, collapse: () => void, delay = 5000) {
  useEffect(() => {
    if (!active) return
    let timer: number | undefined
    const root = document.documentElement
    const start = () => {
      window.clearTimeout(timer)
      timer = window.setTimeout(collapse, delay)
    }
    const stop = () => window.clearTimeout(timer)
    root.addEventListener("mouseleave", start)
    root.addEventListener("mouseenter", stop)
    // Opened with the keyboard, or the pointer is already elsewhere: no mouseleave will come.
    if (!root.matches(":hover")) start()
    return () => {
      stop()
      root.removeEventListener("mouseleave", start)
      root.removeEventListener("mouseenter", stop)
    }
  }, [active, collapse, delay])
}
