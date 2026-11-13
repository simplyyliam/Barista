import { commands } from "@/shared/bindings"
import {
  CalculatorIcon,
  Camera01Icon,
  ContrastIcon,
  Icon,
  ScissorRectangleIcon,
  Timer02Icon,
} from "@barista/ui/icons"
import { cn } from "cn"
import { motion } from "motion/react"
import { PRESS, PRESS_TRANSITION } from "@/shared/motion"
import { useThemeMode } from "./hooks/use-control-center"

/** The card holds at most 4 x 2 actions. The list is data, so a settings screen can fill it later. */
export const MAX_QUICK_ACTIONS = 8

const ACTIONS = [
  { id: "dark-mode", label: "Dark mode", icon: ContrastIcon },
  { id: "timer", label: "Timer", icon: Timer02Icon },
  { id: "calculator", label: "Calculator", icon: CalculatorIcon },
  { id: "camera", label: "Camera", icon: Camera01Icon },
  { id: "screenshot", label: "Screenshot", icon: ScissorRectangleIcon },
] as const

type ActionId = (typeof ACTIONS)[number]["id"]

/** The second card: a grid of 62 px buttons for things you start now (not for things that stay on). */
export function QuickActions() {
  const theme = useThemeMode()

  const run = (id: ActionId) => {
    switch (id) {
      case "dark-mode":
        return void commands.setDarkMode(!theme?.dark)
      case "timer":
        return void commands.openApp("clockTimer")
      case "calculator":
        return void commands.openApp("calculator")
      case "camera":
        return void commands.openApp("camera")
      case "screenshot":
        return void commands.runAction("screenshot", false)
    }
  }

  return (
    <section className="grid grid-cols-4 gap-cc-gap rounded-panel bg-cc-card p-cc-pad">
      {ACTIONS.slice(0, MAX_QUICK_ACTIONS).map((action) => {
        const on = action.id === "dark-mode" && theme?.dark === true
        return (
          <motion.button
            key={action.id}
            type="button"
            whileTap={PRESS}
            transition={PRESS_TRANSITION}
            aria-label={action.label}
            title={action.label}
            aria-pressed={action.id === "dark-mode" ? on : undefined}
            onClick={() => run(action.id)}
            className={cn(
              "flex h-cc-tile w-full items-center justify-center rounded-panel-child text-cc-fg outline-none transition-none",
              "focus-visible:ring-2 focus-visible:ring-cc-fg/60",
              on ? "bg-cc-active" : "bg-cc-tile hover:bg-cc-tile-hover",
            )}
          >
            <Icon icon={action.icon} size={24} />
          </motion.button>
        )
      })}
    </section>
  )
}
