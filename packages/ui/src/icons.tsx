import { HugeiconsIcon } from "@hugeicons/react"
import type { IconSvgElement } from "@hugeicons/react"
import type { LucideIcon } from "lucide-react"

// Apps import icons from here (@barista/ui/icons), never from @hugeicons or lucide directly.
// To add one: re-export it below, so versions and the icon list live in one place.

// Hugeicons keep their own names (they end in "Icon").
export {
  Search01Icon,
  Settings01Icon,
  Wifi01Icon,
  BatteryFullIcon,
  VolumeHighIcon,
} from "@hugeicons/core-free-icons"

// Lucide names clash with Hugeicons' style, so they get a "Lucide" prefix.
export {
  Check as LucideCheck,
  ChevronRight as LucideChevronRight,
} from "lucide-react"

// Only the props both libraries share.
type IconProps = {
  icon: IconSvgElement | LucideIcon | string
  size?: number | string
  color?: string
  strokeWidth?: number
  className?: string
}

/**
 * Renders a Hugeicons or a Lucide icon with the same props.
 * A string is an image URL (e.g. the `data:` URL of a status item icon) and renders as an `<img>`.
 * Example: `<Icon icon={Search01Icon} size={16} />` or `<Icon icon={LucideCheck} size={16} />`
 */
export function Icon({ icon, ...props }: IconProps) {
  if (typeof icon === "string") {
    // Pictures keep their own colours, so color and strokeWidth don't apply.
    const { size = 16, className } = props
    return <img src={icon} width={size} height={size} className={className} alt="" draggable={false} />
  }
  // Hugeicons are plain data (an array); Lucide icons are React components.
  if (Array.isArray(icon)) {
    return <HugeiconsIcon icon={icon as IconSvgElement} {...props} />
  }
  const Lucide = icon as LucideIcon
  return <Lucide {...props} />
}
