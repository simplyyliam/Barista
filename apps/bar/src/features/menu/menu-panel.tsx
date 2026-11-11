import { motion } from "motion/react"
import { useRef, type ReactNode } from "react"
import { useMenuSize } from "./hooks/use-menu-size"

type MenuPanelProps = {
  /** Fading out. Opening never animates: the menu must be there on the first frame. */
  closing: boolean
  /** The window has the system acrylic backdrop; if not, draw an opaque panel. */
  acrylic: boolean
  label: string
  activeId?: string
  onSize: (width: number, height: number) => void
  children: ReactNode
}

/**
 * The glass of one menu window. The window itself is the panel (Windows draws the blur, the
 * corners and the shadow), so this only adds the tint, a hairline edge and the close fade.
 * Give it a new `key` for every open: it starts fresh and fully visible, with no enter animation.
 */
export function MenuPanel({ closing, acrylic, label, activeId, onSize, children }: MenuPanelProps) {
  const measureRef = useRef<HTMLDivElement>(null)
  useMenuSize(measureRef, onSize)

  return (
    <motion.div
      initial={false}
      animate={{ opacity: closing ? 0 : 1 }}
      transition={closing ? { duration: 0.15, ease: "easeOut" } : { duration: 0 }}
      data-acrylic={acrylic}
      style={{ pointerEvents: closing ? "none" : "auto" }}
      onContextMenu={(event) => event.preventDefault()}
      className="fixed inset-0 cursor-default overflow-x-hidden overflow-y-auto rounded-[var(--window-radius,8px)] bg-menu-opaque font-bar text-[13px] leading-none text-menu-fg shadow-[inset_0_0_0_1px_var(--color-menu-ring)] select-none data-[acrylic=true]:bg-menu-tint"
    >
      {/* This inner box is what we measure: the window is exactly as big as it. */}
      <div
        ref={measureRef}
        role="menu"
        aria-label={label}
        aria-activedescendant={activeId}
        className="w-max max-w-menu-max-w min-w-menu-min-w p-menu-pad"
      >
        {children}
      </div>
    </motion.div>
  )
}
