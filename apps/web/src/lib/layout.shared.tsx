import type { BaseLayoutProps } from "fumadocs-ui/layouts/shared"
import { ThemeToggle } from "@/components/theme-toggle"

// Options shared by every Fumadocs layout (the docs sidebar today).
export function baseOptions(): BaseLayoutProps {
  // The docs use the same animated switch as the landing page.
  return { nav: { title: "Barista" }, themeSwitch: { component: <ThemeToggle /> } }
}
