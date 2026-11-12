import { useEffect } from "react"
import { commands } from "@/shared/bindings"
import { PanelBody, PanelFrame, useOpenPanel } from "@/features/panel"

/**
 * The panel window (label `panel`): one window, reused for every system panel. Rust decides when
 * it opens and where it goes; this only draws the route named in `panelChanged`, reports its size
 * and asks Rust to close on Esc (ADR 0016).
 */
export default function PanelWindow() {
  const open = useOpenPanel()

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") void commands.closePanel()
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [])

  if (!open) return null
  const key = `${open.name}|${open.anchor}`
  return (
    <PanelFrame key={key} acrylic={open.acrylic} chrome={open.chrome} openKey={key}>
      <PanelBody name={open.name} />
    </PanelFrame>
  )
}
