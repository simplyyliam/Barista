import { useEffect } from "react"
import { commands } from "@/shared/bindings"

/**
 * Makes the bar window as tall as the `--menubar-height` token (styles/tokens.css, written in px),
 * so the CSS value is the one place to change it. Rust clamps the value, saves it and re-places
 * the bar; nothing happens when it already matches. Rust may not be ready when the page loads
 * (`getBarInfo` answers with an error then), so it asks again a few times.
 */
export function useBarHeight() {
  useEffect(() => {
    const wanted = Math.round(parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--menubar-height")))
    if (!Number.isFinite(wanted)) return
    let cancelled = false
    let timer: ReturnType<typeof setTimeout> | undefined

    const check = (triesLeft: number) => {
      void commands.getBarInfo().then((result) => {
        if (cancelled) return
        if (result.status === "ok") {
          if (result.data.height !== wanted) void commands.setBarHeight(wanted)
        } else if (triesLeft > 0) {
          timer = setTimeout(() => check(triesLeft - 1), 500)
        }
      })
    }
    check(5)

    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [])
}
