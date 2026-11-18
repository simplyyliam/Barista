import { useEffect, useRef, useState, type ReactNode } from "react"
import { cn } from "cn"

/**
 * A scrolling area with black fades at the top and bottom edge, like ChatGPT's. Each fade only shows
 * while there is more to scroll that way. Scroll and resize events only, no polling.
 */
export function ScrollFade({
  children,
  className,
  innerClassName,
  label,
  fade = "h-10",
}: {
  children: ReactNode
  className?: string
  innerClassName?: string
  label?: string
  fade?: string
}) {
  const box = useRef<HTMLDivElement>(null)
  const [edge, setEdge] = useState({ top: false, bottom: false })

  useEffect(() => {
    const el = box.current
    if (!el) return
    const update = () => {
      const top = el.scrollTop > 2
      const bottom = el.scrollTop + el.clientHeight < el.scrollHeight - 2
      setEdge((prev) => (prev.top === top && prev.bottom === bottom ? prev : { top, bottom }))
    }
    update()
    el.addEventListener("scroll", update, { passive: true })
    // The page's height changes when a preview opens or the window is resized.
    const observer = new ResizeObserver(update)
    observer.observe(el)
    if (el.firstElementChild) observer.observe(el.firstElementChild)
    return () => {
      el.removeEventListener("scroll", update)
      observer.disconnect()
    }
  }, [])

  const shade = "pointer-events-none absolute inset-x-0 z-10 transition-opacity duration-200"
  return (
    <div className={cn("relative min-h-0", className)}>
      <div
        ref={box}
        aria-label={label}
        className={cn("h-full overflow-y-auto [scrollbar-width:none]", innerClassName)}
      >
        <div>{children}</div>
      </div>
      <div
        aria-hidden="true"
        className={cn(shade, fade, "top-0 bg-gradient-to-b from-black to-transparent", edge.top ? "opacity-100" : "opacity-0")}
      />
      <div
        aria-hidden="true"
        className={cn(shade, fade, "bottom-0 bg-gradient-to-t from-black to-transparent", edge.bottom ? "opacity-100" : "opacity-0")}
      />
    </div>
  )
}
