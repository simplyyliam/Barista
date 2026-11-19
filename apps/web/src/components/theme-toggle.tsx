"use client"

import { useTheme } from "fumadocs-ui/provider/base"
import { useEffect, useState } from "react"
import { flushSync } from "react-dom"

// One theme switch for the landing page and the docs. A click does not swap the colours: the new theme
// opens as a circle from the button and grows across the page (View Transitions), like a light coming
// on at the switch. Browsers without it, and people who asked for less motion, get an instant change.

const SPRING = "cubic-bezier(0.22, 1, 0.36, 1)"

export function ThemeToggle({ className = "" }: { className?: string }) {
  const { resolvedTheme, setTheme } = useTheme()
  // The server does not know the theme: render the same markup there, then fill in on the client.
  const [mounted, setMounted] = useState(false)
  useEffect(() => setMounted(true), [])
  const dark = mounted && resolvedTheme === "dark"

  function toggle(event: React.MouseEvent<HTMLButtonElement>) {
    const next = dark ? "light" : "dark"
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches
    if (reduced || !document.startViewTransition) return setTheme(next)

    const box = event.currentTarget.getBoundingClientRect()
    const x = box.left + box.width / 2
    const y = box.top + box.height / 2
    const radius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y))

    const transition = document.startViewTransition(() => flushSync(() => setTheme(next)))
    void transition.ready.then(() =>
      document.documentElement.animate(
        { clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${radius}px at ${x}px ${y}px)`] },
        { duration: 650, easing: SPRING, pseudoElement: "::view-transition-new(root)" },
      ),
    )
  }

  return (
    <button
      type="button"
      onClick={toggle}
      role="switch"
      aria-checked={dark}
      aria-label="Dark theme"
      title={dark ? "Switch to light" : "Switch to dark"}
      data-dark={dark}
      className={`theme-toggle ${className}`}
    >
      {/* The sun and the moon share one spot: the one leaving turns and shrinks away, the other arrives. */}
      <svg viewBox="0 0 24 24" aria-hidden="true" className="theme-toggle__sun">
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2.5v2.2M12 19.3v2.2M4.5 4.5l1.6 1.6M17.9 17.9l1.6 1.6M2.5 12h2.2M19.3 12h2.2M4.5 19.5l1.6-1.6M17.9 6.1l1.6-1.6" />
      </svg>
      <svg viewBox="0 0 24 24" aria-hidden="true" className="theme-toggle__moon">
        <path d="M20 14.5A8 8 0 0 1 9.5 4 8.5 8.5 0 1 0 20 14.5z" />
      </svg>
    </button>
  )
}
