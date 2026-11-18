"use client"

import { useState } from "react"
import { WindowsGlyph } from "./icons"
import { DOWNLOAD_URL, NAV_LINKS } from "./site"

// The nav is a menu bar, like the product: brand on the left, links as bar items, status icons on the right.
export function Nav() {
  const [open, setOpen] = useState(false)
  return (
    <header className="bar">
      <div className="bar__inner">
        <a href="#top" className="bar__item brand">Barista</a>
        {NAV_LINKS.map((l) => (
          <a key={l.href} href={l.href} className="bar__item bar__link">
            {l.label}
          </a>
        ))}
        <span className="bar__sp" />
        <span className="bar__status" aria-hidden="true">
          <svg viewBox="0 0 24 24"><path d="M2.5 9a14 14 0 0 1 19 0M5.5 12.5a9.5 9.5 0 0 1 13 0M8.7 16a5 5 0 0 1 6.6 0" /><circle cx="12" cy="19" r=".6" /></svg>
          <svg viewBox="0 0 24 24"><rect x="3" y="8" width="16" height="9" rx="2.5" /><path d="M21 11v3" /></svg>
          <span className="bar__time">22:00</span>
        </span>
        <a href={DOWNLOAD_URL} className="btn btn--sm btn--primary bar__cta">
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
      <div id="drawer" className="drawer" data-open={open}>
        {NAV_LINKS.map((l) => (
          <a key={l.href} href={l.href} onClick={() => setOpen(false)}>
            {l.label}
          </a>
        ))}
        <a href={DOWNLOAD_URL} className="btn btn--sm btn--primary" style={{ alignSelf: "flex-start" }}>
          <WindowsGlyph />
          Download
        </a>
      </div>
    </header>
  )
}
