import type { ComponentProps, ReactNode } from "react"
import { motion, type HTMLMotionProps } from "motion/react"
import { PRESS, PRESS_TRANSITION } from "@/shared/motion"
import { cn } from "cn"
import { Icon } from "@barista/ui/icons"

type TileProps = Omit<HTMLMotionProps<"button">, "children"> & {
  icon: ComponentProps<typeof Icon>["icon"]
  /** With a label the tile is meant for two columns (136 px in App.png), without one for one (62 px). The grid decides. */
  label?: string
  sub?: string
  /** Black, like the Wi-Fi and Bluetooth tiles in App.png. */
  active?: boolean
  children?: ReactNode
}

/**
 * One tile of the control center grid. Every tile fills its cell (62 px high); a labelled one has
 * 17 px at the sides.
 */
export function Tile({ icon, label, sub, active = false, className, children, ...props }: TileProps) {
  return (
    <motion.button
      type="button"
      aria-pressed={active}
      whileTap={props.disabled ? undefined : PRESS}
      transition={PRESS_TRANSITION}
      {...props}
      className={cn(
        "flex h-cc-tile w-full items-center rounded-panel-child text-cc-fg outline-none transition-none",
        "focus-visible:ring-2 focus-visible:ring-cc-fg/60 disabled:opacity-40",
        active ? "bg-cc-active" : "bg-cc-tile enabled:hover:bg-cc-tile-hover",
        label ? "justify-start gap-2.5 px-cc-label-pad text-left" : "justify-center",
        className,
      )}
    >
      <Icon icon={icon} size={label ? 20 : 24} className="shrink-0" />
      {label && (
        <span className="flex min-w-0 flex-col leading-tight">
          <span className="truncate text-[11px] font-medium">{label}</span>
          {sub && <span className="truncate text-[10px] text-cc-fg-muted">{sub}</span>}
        </span>
      )}
      {children}
    </motion.button>
  )
}
