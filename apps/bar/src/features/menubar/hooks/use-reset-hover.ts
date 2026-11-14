import { useEffect } from "react"
import { events } from "@/shared/bindings"
import { resetHover } from "@/shared/stores/hover"

/**
 * Clears the pills' hover tint whenever the pointer may have gone into another window: a panel,
 * menu or popover opening or closing, or the bar losing focus. Call once, in the bar.
 */
export function useResetHover() {
  useEffect(() => {
    const off = [
      events.panelChanged.listen(resetHover),
      events.popoverChanged.listen(resetHover),
      events.menuOpen.listen(resetHover),
      events.menuClose.listen(resetHover),
    ]
    window.addEventListener("blur", resetHover)
    return () => {
      window.removeEventListener("blur", resetHover)
      off.forEach((stop) => void stop.then((fn) => fn()))
    }
  }, [])
}
