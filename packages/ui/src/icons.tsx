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
  VolumeMute01Icon,
  BluetoothIcon,
  AirplaneModeIcon,
  Moon02Icon,
  BatteryEcoChargingIcon,
  HotspotIcon,
  Sun03Icon,
  ShutDownIcon,
  ContrastIcon,
  Timer02Icon,
  CalculatorIcon,
  Camera01Icon,
  ScissorRectangleIcon,
  PreviousIcon,
  NextIcon,
  PlayIcon,
  PauseIcon,
  MusicNote01Icon,
  ArrowLeft01Icon,
  SlidersHorizontalIcon,
  LockIcon,
  UserIcon,
  Tick02Icon,
  Coffee03Icon,
} from "@hugeicons/core-free-icons"

// Lucide names clash with Hugeicons' style, so they get a "Lucide" prefix.
export {
  Bell as LucideBell,
  Check as LucideCheck,
  ChevronDown as LucideChevronDown,
  ChevronLeft as LucideChevronLeft,
  ChevronRight as LucideChevronRight,
  Minus as LucideMinus,
  Plus as LucidePlus,
  X as LucideX,
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
    const { size, className } = props
    // Status item images carry their own size (the SDK sets width and height: 16 x 16 for square
    // icons, about 25 x 12 for the battery). Draw them as they are, never taller than the 22 px
    // icon area. An explicit `size` (menu entries pass 16) still wins.
    return (
      <img
        src={icon}
        {...(size !== undefined && { width: size, height: size })}
        style={{ maxHeight: 22 }}
        className={className}
        alt=""
        draggable={false}
      />
    )
  }
  // Hugeicons are plain data (an array); Lucide icons are React components.
  if (Array.isArray(icon)) {
    return <HugeiconsIcon icon={icon as IconSvgElement} {...props} />
  }
  const Lucide = icon as LucideIcon
  return <Lucide {...props} />
}
