"use client"

import { useEffect } from "react"

// Adds the `js` class (content is never hidden without JS), fades each [data-reveal] in once,
// and makes the bean's pupils look at the cursor (the page's one playful interaction).
export function RevealController() {
  useEffect(() => {
    const root = document.querySelector<HTMLElement>(".sl")
    if (!root) return
    root.classList.add("js")

    // CSS vars can't reach inside a cursor image, so rebuild it from --bean here.
    const syncCursor = () => {
      const bean = getComputedStyle(root).getPropertyValue("--bean").trim()
      if (bean) {
        const cursor = (w: number) =>
          `url("data:image/svg+xml,${encodeURIComponent(
            `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 140 160' width='${w}' height='${Math.round((w * 160) / 140)}'><ellipse cx='70' cy='82' rx='52' ry='68' transform='rotate(-14 70 82)' fill='${bean}' stroke='#fff' stroke-width='8'/><rect x='38' y='62' width='24' height='38' rx='12' fill='#fff'/><rect x='78' y='62' width='24' height='38' rx='12' fill='#fff'/><circle cx='50' cy='84' r='6.5' fill='#0a0a0a'/><circle cx='90' cy='84' r='6.5' fill='#0a0a0a'/></svg>`,
          )}")`
        root.style.setProperty("--cur-bean", cursor(26))
        root.style.setProperty("--cur-bean-link", cursor(32))
      }
    }
    syncCursor()
    // CSS hot reload doesn't re-run effects, so re-sync when you come back to the tab.
    window.addEventListener("focus", syncCursor)

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
      window.removeEventListener("focus", syncCursor)
      io.disconnect()
      window.removeEventListener("pointermove", look)
      cancelAnimationFrame(frame)
    }
  }, [])
  return null
}
