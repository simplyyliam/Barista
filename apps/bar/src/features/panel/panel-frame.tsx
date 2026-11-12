import { useRef, type ReactNode } from "react"
import { usePanelSize } from "./hooks/use-panel-size"

type PanelFrameProps = {
  /** The window has the system acrylic backdrop; if not, draw an opaque panel. */
  acrylic: boolean
  /** New for every open, so the size is reported again. */
  openKey: string
  children: ReactNode
}

/** The glass of the panel window. The window itself is the panel (Windows draws blur, corners, shadow). */
export function PanelFrame({ acrylic, openKey, children }: PanelFrameProps) {
  const measureRef = useRef<HTMLDivElement>(null)
  usePanelSize(measureRef, openKey)

  return (
    <div
      data-acrylic={acrylic}
      onContextMenu={(event) => event.preventDefault()}
      className="fixed inset-0 cursor-default overflow-hidden rounded-[var(--window-radius,8px)] bg-menu-opaque font-bar text-menu-fg shadow-[inset_0_0_0_1px_var(--color-menu-ring)] select-none data-[acrylic=true]:bg-menu-tint"
    >
      <div ref={measureRef} className="w-max min-w-64">
        {children}
      </div>
    </div>
  )
}
