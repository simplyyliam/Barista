import type { CSSProperties, ReactNode, Ref } from "react"
import { cn } from "cn"
import type { Settings } from "@/shared/stores/settings"
import wallpaper from "@/assets/preview-wallpaper.jpg"

/**
 * The picture the editors sit on (the design: a wallpaper with the bar or the control center on
 * it). It carries the saved look as CSS variables, so the bar drawn inside looks like the real one:
 * tone, font, text size, solid background and icon style.
 */
export function PreviewFrame({
  settings,
  height,
  children,
  frameRef,
  className,
}: {
  settings: Settings
  height: number
  children: ReactNode
  frameRef?: Ref<HTMLDivElement>
  className?: string
}) {
  const look = settings.menuBar.appearance
  const style: CSSProperties & Record<string, string> = {
    backgroundImage: `url(${wallpaper})`,
    height: `${height}px`,
    "--text-bar": `${look.fontSize}px`,
    "--bar-bg-opacity": `${look.opacity}%`,
  }
  if (look.font) style["--font-bar"] = `"${look.font}", "Segoe UI Variable", system-ui, sans-serif`
  return (
    <div
      ref={frameRef}
      // Light wallpaper = dark text, unless the user forced the dark tone.
      data-tone={look.tone === "dark" ? "dark" : "light"}
      data-icon-style={look.iconStyle}
      data-bar-background={look.background}
      style={style}
      className={cn("relative overflow-hidden rounded-2xl bg-cover bg-center", className)}
    >
      {children}
    </div>
  )
}
