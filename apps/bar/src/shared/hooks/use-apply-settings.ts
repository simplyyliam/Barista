import { useEffect } from "react"
import { useSettings } from "@/shared/stores/settings"

/**
 * Turns the saved appearance into CSS variables and attributes on `<html>`, so every window that
 * calls it (the bar, the panels) looks the same. The tokens live in `styles/tokens.css`; this only
 * overrides them. An empty font keeps the token's default.
 */
export function useApplySettings() {
  const settings = useSettings()
  const bar = settings?.menuBar
  const look = bar?.appearance

  useEffect(() => {
    if (!bar) return
    const root = document.documentElement.style
    root.setProperty("--menubar-height", `${bar.height}px`)
  }, [bar?.height])

  useEffect(() => {
    if (!look) return
    const root = document.documentElement
    // The font name is validated in Rust (letters, digits, space . - _), so quoting is enough.
    if (look.font) root.style.setProperty("--font-bar", `"${look.font}", "Segoe UI Variable", system-ui, sans-serif`)
    else root.style.removeProperty("--font-bar")
    root.style.setProperty("--text-bar", `${look.fontSize}px`)
    root.style.setProperty("--bar-bg-opacity", `${look.opacity}%`)
    root.dataset.iconStyle = look.iconStyle
    root.dataset.barBackground = look.background
  }, [look?.font, look?.fontSize, look?.opacity, look?.iconStyle, look?.background])
}
