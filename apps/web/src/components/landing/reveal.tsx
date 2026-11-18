"use client"

import { useEffect } from "react"

// Adds the `js` class (so content is never hidden without JS), then fades each [data-reveal] in once.
export function RevealController() {
  useEffect(() => {
    const root = document.querySelector(".sl")
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
      io.observe(el)
    })
    return () => io.disconnect()
  }, [])
  return null
}
