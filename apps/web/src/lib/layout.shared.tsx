import type { BaseLayoutProps } from "fumadocs-ui/layouts/shared"

// Options shared by every Fumadocs layout (the docs sidebar today).
export function baseOptions(): BaseLayoutProps {
  return { nav: { title: "Barista" } }
}
