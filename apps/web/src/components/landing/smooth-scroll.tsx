"use client"

import { useEffect } from "react"
import Lenis from "lenis"

// Smooth (inertia) scrolling for the landing page only. Skipped for reduced motion.
export function SmoothScroll() {
  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return
    const lenis = new Lenis({ anchors: true })
    let frame = requestAnimationFrame(function raf(time) {
      lenis.raf(time)
      frame = requestAnimationFrame(raf)
    })
    return () => {
      cancelAnimationFrame(frame)
      lenis.destroy()
    }
  }, [])
  return null
}
