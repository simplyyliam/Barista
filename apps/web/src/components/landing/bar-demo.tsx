"use client"

import { useEffect, useRef, useState } from "react"
import { burst } from "./demo"

// The real Barista bar and panels, framed (ADR 0037). The page in /demo is built from apps/bar and runs
// on pretend data, so what visitors click here is the actual product UI, not a drawing of it.
// The frame has a fixed design size and is scaled to fit its box, so phones get the same picture.

const SIZES: Record<string, { width: number; height: number; maxScale?: number }> = {
  // The whole desktop: bar on top, a panel open under it.
  desktop: { width: 900, height: 680 },
  // Just the panel, for the small tiles.
  panel: { width: 360, height: 660 },
  // Just the bar with one item on it (the SDK example). It is only 28px tall, so it is drawn a bit bigger.
  bar: { width: 520, height: 28, maxScale: 1.5 },
  // The bar with the tray panel open under its arrow, and room for an icon's menu.
  tray: { width: 520, height: 240, maxScale: 1.4 },
}

export function BarDemo({
  scene,
  size = "desktop",
  label,
  className = "",
}: {
  /** What is open at the start (see apps/bar/demo/stage.tsx). `sdk` is the bar with just the SDK item. */
  scene: "notifications" | "control" | "sdk" | "tray"
  size?: "desktop" | "panel" | "bar" | "tray"
  label: string
  className?: string
}) {
  const box = useRef<HTMLDivElement>(null)
  const frame = useRef<HTMLIFrameElement>(null)
  const [scale, setScale] = useState(1)
  // The control center is shorter than the notifications panel (which has the calendar under it).
  const height = size === "panel" && scene === "control" ? 560 : SIZES[size]!.height
  const { width } = SIZES[size]!
  const maxScale = SIZES[size]!.maxScale ?? 1

  useEffect(() => {
    const el = box.current
    if (!el) return
    const fit = () => setScale(Math.min(maxScale, el.clientWidth / width))
    fit()
    const observer = new ResizeObserver(fit)
    observer.observe(el)
    return () => observer.disconnect()
  }, [width, maxScale])

  // The SDK counter in the demo says "milestone" every fifth click: confetti from where it sits.
  useEffect(() => {
    if (scene !== "sdk") return
    const onMessage = (e: MessageEvent) => {
      const d = e.data as { type?: string; event?: string; x?: number; y?: number } | null
      if (e.origin !== location.origin || e.source !== frame.current?.contentWindow) return
      if (d?.type !== "barista-demo" || d.event !== "milestone" || typeof d.x !== "number" || typeof d.y !== "number") return
      const r = frame.current.getBoundingClientRect()
      burst(r.left + d.x * scale, r.top + d.y * scale, 10)
    }
    window.addEventListener("message", onMessage)
    return () => window.removeEventListener("message", onMessage)
  }, [scene, scale])

  return (
    <div
      ref={box}
      className={`bar-demo ${className}`}
      style={{ aspectRatio: `${width} / ${height}`, maxWidth: width * maxScale }}
    >
      <iframe
        ref={frame}
        title={label}
        src={`/demo/index.html?scene=${scene}${size === "panel" ? "&bare=1" : ""}`}
        loading="lazy"
        width={width}
        height={height}
        style={{ transform: `scale(${scale})`, transformOrigin: "0 0" }}
      />
    </div>
  )
}
