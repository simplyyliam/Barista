"use client"

import { useEffect, useLayoutEffect, useRef, useState } from "react"
import { ThemeToggle } from "../theme-toggle"
import { BeanGlyph, WindowsGlyph } from "./icons"
import { DOWNLOAD_URL, NAV_LINKS } from "./site"

// The nav is the product: a floating black menu bar. Brand on the left, links as bar items with one
// capsule that glides to whatever you hover (and rests on the section you are reading), status icons
// and a live clock on the right.
export function Nav() {
  const [open, setOpen] = useState(false)
  const [hover, setHover] = useState<number | null>(null)
  const [active, setActive] = useState<number | null>(null)
  const [time, setTime] = useState("")
  const [shrunk, setShrunk] = useState(false)
  const links = useRef<HTMLDivElement>(null)
  const [pill, setPill] = useState({ x: 0, w: 0, on: false })

  useEffect(() => {
    const tick = () => setTime(new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false }))
    tick()
    const t = setInterval(tick, 15_000)
    return () => clearInterval(t)
  }, [])

  // Which section is on screen decides where the capsule rests.
  useEffect(() => {
    const targets = NAV_LINKS.map((l, i) => ({ i, el: l.href.startsWith("#") ? document.querySelector(l.href) : null })).filter(
      (t): t is { i: number; el: Element } => !!t.el,
    )
    const io = new IntersectionObserver(
      (entries) =>
        entries.forEach((e) => {
          const hit = targets.find((t) => t.el === e.target)
          if (hit && e.isIntersecting) setActive(hit.i)
        }),
      { rootMargin: "-45% 0px -50% 0px" },
    )
    targets.forEach((t) => io.observe(t.el))
    const top = () => {
      setShrunk(window.scrollY > 40)
      if (window.scrollY < 200) setActive(null)
    }
    top()
    window.addEventListener("scroll", top, { passive: true })
    return () => {
      io.disconnect()
      window.removeEventListener("scroll", top)
    }
  }, [])

  const target = hover ?? active
  useLayoutEffect(() => {
    const el = target === null ? null : links.current?.children[target + 1]
    if (!(el instanceof HTMLElement)) return setPill((p) => ({ ...p, on: false }))
    setPill({ x: el.offsetLeft, w: el.offsetWidth, on: true })
  }, [target])

  return (
    <header className="bar" data-shrunk={shrunk}>
      <div className="bar__inner">
        <a href="#top" className="bar__item brand">
          <span className="brand__mark"><BeanGlyph /></span>
          Barista
        </a>
        <div className="bar__links" ref={links} onPointerLeave={() => setHover(null)} onBlur={() => setHover(null)}>
          <span
            className="bar__pill"
            data-rest={hover === null}
            style={{ transform: `translateX(${pill.x}px)`, width: pill.w, opacity: pill.on ? 1 : 0 }}
          />
          {NAV_LINKS.map((l, i) => (
            <a
              key={l.href}
              href={l.href}
              className="bar__item bar__link"
              data-active={active === i}
              onPointerEnter={() => setHover(i)}
              onFocus={() => setHover(i)}
            >
              {l.label}
            </a>
          ))}
        </div>
        <div className="bar__right">
        <span className="bar__status" aria-hidden="true">
          <svg viewBox="0 0 24 24"><path d="M2.5 9a14 14 0 0 1 19 0M5.5 12.5a9.5 9.5 0 0 1 13 0M8.7 16a5 5 0 0 1 6.6 0" /><circle cx="12" cy="19" r=".6" /></svg>
          <svg viewBox="0 0 24 24"><rect x="3" y="8" width="16" height="9" rx="2.5" /><path d="M21 11v3" /><rect className="bar__charge" x="5" y="10" width="12" height="5" rx="1" fill="currentColor" stroke="none" /></svg>
          <span className="bar__time">{time || "--:--"}</span>
        </span>
        <ThemeToggle className="bar__theme" />
        <a href={DOWNLOAD_URL} className="btn btn--sm btn--white bar__cta">
          <WindowsGlyph />
          Download
        </a>
        <button
          type="button"
          className="bar__item bar__menu"
          aria-expanded={open}
          aria-controls="drawer"
          onClick={() => setOpen((o) => !o)}
        >
          {open ? "Close" : "Menu"}
        </button>
        </div>
      </div>
      <div id="drawer" className="drawer" data-open={open}>
        {NAV_LINKS.map((l) => (
          <a key={l.href} href={l.href} onClick={() => setOpen(false)}>
            {l.label}
          </a>
        ))}
        <a href={DOWNLOAD_URL} className="btn btn--sm btn--white" style={{ alignSelf: "flex-start" }}>
          <WindowsGlyph />
          Download
        </a>
      </div>
    </header>
  )
}
