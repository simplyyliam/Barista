"use client"

import { useState } from "react"
import { AppIcon, WindowsGlyph } from "./icons"
import { DOWNLOAD_URL, NAV_LINKS } from "./site"

export function Nav() {
  const [open, setOpen] = useState(false)
  return (
    <header className="nav">
      <div className="nav__inner">
        <a href="#top" className="brand">
          <AppIcon />
          Barista
        </a>
        <nav className="nav__links" aria-label="Main">
          {NAV_LINKS.map((l) => (
            <a key={l.href} href={l.href}>
              {l.label}
            </a>
          ))}
        </nav>
        <a href={DOWNLOAD_URL} className="btn btn--sm btn--light nav__cta">
          <WindowsGlyph />
          Download
        </a>
        <button
          type="button"
          className="menu-btn"
          aria-expanded={open}
          aria-controls="nav-drawer"
          onClick={() => setOpen((o) => !o)}
        >
          {open ? "Close" : "Menu"}
        </button>
      </div>
      <div id="nav-drawer" className="nav__drawer" data-open={open}>
        {NAV_LINKS.map((l) => (
          <a key={l.href} href={l.href} onClick={() => setOpen(false)}>
            {l.label}
          </a>
        ))}
        <a href={DOWNLOAD_URL} className="btn btn--sm btn--light" style={{ alignSelf: "flex-start" }}>
          <WindowsGlyph />
          Download
        </a>
      </div>
    </header>
  )
}
