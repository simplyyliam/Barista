import { commands, events } from "@/shared/bindings"

/**
 * The tone of the WALLPAPER behind the bar, measured by Rust.
 * "light" wallpaper = dark content, "dark" wallpaper = light content.
 * It is also written to <html data-tone="...">, which switches the tokens in styles/tokens.css.
 */
type BarTone = "light" | "dark"

// Light is also the CSS default, so nothing flashes before Rust answers.
let tone: BarTone = "light"

function setTone(next: BarTone) {
  document.documentElement.dataset.tone = next
  if (next === tone) return
  tone = next
}

/**
 * Starts following the bar tone. Call once, before rendering (main.tsx does).
 *
 * @example
 * void startBarTone()
 */
export async function startBarTone() {
  setTone(tone)
  let gotEvent = false
  try {
    // Listen first, then read, so a change in between is not lost.
    await events.barAppearanceChanged.listen((e) => {
      gotEvent = true
      setTone(e.payload.tone)
    })
    const current = await commands.getBarAppearance()
    // An event that arrived while we were asking is newer than this answer.
    if (!gotEvent) setTone(current.tone)
  } catch (error) {
    // Keep the default tone; the bar still works, it just cannot follow the wallpaper.
    console.error("bar tone unavailable:", error)
  }
}
