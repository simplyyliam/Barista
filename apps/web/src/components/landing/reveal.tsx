"use client"

import { useEffect } from "react"

// Adds the `js` class (content is never hidden without JS), fades each [data-reveal] in once,
// and makes the bean's pupils look at the cursor (the page's one playful interaction).
export function RevealController() {
  useEffect(() => {
    const root = document.querySelector<HTMLElement>(".sl")
    if (!root) return
    root.classList.add("js")
    const io = new IntersectionObserver(
      (entries) =>
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add("in")
            io.unobserve(e.target)
          }
        }),
      { threshold: 0.12 },
    )
    root.querySelectorAll<HTMLElement>("[data-reveal]").forEach((el) => {
      el.style.transitionDelay = `${el.dataset.delay ?? 0}ms`
      // Already on screen at load: show it now instead of waiting for the observer.
      if (el.getBoundingClientRect().top < window.innerHeight) el.classList.add("in")
      else io.observe(el)
    })

    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches
    let frame = 0
    const look = (ev: PointerEvent) => {
      if (reduce || frame) return
      frame = requestAnimationFrame(() => {
        frame = 0
        root.querySelectorAll<HTMLElement>(".bean").forEach((b) => {
          const r = b.getBoundingClientRect()
          const dx = ev.clientX - (r.left + r.width / 2)
          const dy = ev.clientY - (r.top + r.height / 2)
          const d = Math.hypot(dx, dy) || 1
          const k = Math.min(1, d / 200) * 5
          b.style.setProperty("--px", `${(dx / d) * k}px`)
          b.style.setProperty("--py", `${(dy / d) * k}px`)
        })
      })
    }
    window.addEventListener("pointermove", look)
    return () => {
      io.disconnect()
      window.removeEventListener("pointermove", look)
      cancelAnimationFrame(frame)
    }
  }, [])
  return null
}
