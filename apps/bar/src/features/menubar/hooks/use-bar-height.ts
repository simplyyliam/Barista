import { useEffect } from "react"
import { commands } from "@/shared/bindings"

/**
 * Makes the bar window as tall as the `--menubar-height` token (styles/tokens.css), so the CSS
 * value is the one place to change it. Rust clamps the value, saves it and re-places the bar;
 * nothing happens when it already matches.
 */
export function useBarHeight() {
  useEffect(() => {
    const wanted = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--menubar-height"))
    if (!Number.isFinite(wanted)) return
    void commands.getBarInfo().then((result) => {
      if (result.status === "ok" && result.data.height !== Math.round(wanted)) {
        void commands.setBarHeight(Math.round(wanted))
      }
    })
  }, [])
}
