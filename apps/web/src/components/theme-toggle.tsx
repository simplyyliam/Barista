"use client"

import { useTheme } from "fumadocs-ui/provider/base"
import { useEffect, useState } from "react"

// One theme switch for the landing page and the docs. A click does not snap the colours: the new theme
// spreads out from the button with a soft edge (View Transitions + a feathered mask), slowly enough to
// feel like light moving across the page rather than a flash. Browsers without it, and people who asked
// for less motion, get an instant change.

const DURATION = 1300
const EASE = "cubic-bezier(0.45, 0, 0.2, 1)" // eases in and out: no sudden start, no sudden stop

/**
 * The page's colours depend only on the class on <html>, so that is all the click changes, inside the
 * view transition's callback. Re-rendering the React tree there (what `setTheme` does) blocked the click for
 * over 100 ms and made the first frames of the animation stutter. The theme provider is told afterwards,
 * when nothing is moving, so its state and the saved choice catch up.
 */
function paint(theme: "light" | "dark") {
  const root = document.documentElement
  root.classList.remove("light", "dark")
  root.classList.add(theme)
  root.style.colorScheme = theme
}

export function ThemeToggle({ className = "" }: { className?: string }) {
  const { resolvedTheme, setTheme } = useTheme()
  // The server does not know the theme: render the same markup there, then fill in on the client.
  const [dark, setDark] = useState(false)
  useEffect(() => setDark(resolvedTheme === "dark"), [resolvedTheme])

  function toggle(event: React.MouseEvent<HTMLButtonElement>) {
    const next = dark ? "light" : "dark"
    setDark(next === "dark") // only this button re-renders now: its icon turns at once
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches
    if (reduced || !document.startViewTransition) return setTheme(next)

    const box = event.currentTarget.getBoundingClientRect()
    const x = box.left + box.width / 2
    const y = box.top + box.height / 2
    const radius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y))
    // The mask is a soft-edged disc (see global.css): fully opaque inside 60% of its radius, so it has to be
    // big enough that the opaque part covers the farthest corner when it is done.
    const size = radius * 3.6

    // During a view transition the page is two still pictures, so the button's own icon animation would
    // freeze and then jump. Naming the button gives it a layer of its own that stays live above the reveal
    // (see global.css). Only while it runs: two toggles on a page must never share the name.
    const button = event.currentTarget
    button.style.viewTransitionName = "theme-toggle"
    // Colour transitions would run inside the new picture and blur the reveal's edge, so they pause meanwhile.
    document.documentElement.classList.add("theme-switching")
    const transition = document.startViewTransition(() => paint(next))
    void transition.finished.finally(() => {
      button.style.viewTransitionName = ""
      document.documentElement.classList.remove("theme-switching")
      const later = window.requestIdleCallback ?? ((fn: () => void) => setTimeout(fn, 50))
      later(() => setTheme(next))
    })
    void transition.ready.then(() =>
      document.documentElement.animate(
        {
          maskSize: ["0px 0px", `${size}px ${size}px`],
          maskPosition: [`${x}px ${y}px`, `${x - size / 2}px ${y - size / 2}px`],
        },
        // fill: the last frame must hold until the transition ends, or the mask falls back to its resting
        // state (a vignette) for a frame and the page flashes.
        { duration: DURATION, easing: EASE, fill: "forwards", pseudoElement: "::view-transition-new(root)" },
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
