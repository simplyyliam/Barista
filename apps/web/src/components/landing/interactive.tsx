"use client"

import { useEffect, useRef, useState } from "react"
import { burst } from "./demo"
import { WindowsGlyph } from "./icons"
import { DOWNLOAD_URL } from "./site"

// Small interactive pieces for the landing page. Each one mirrors something the real bar does.

const NOTES = [
  { b: "Messages", t: "You may have a new message" },
  { b: "Snipping Tool", t: "Screenshot copied to clipboard. Saved to your screenshots folder." },
  { b: "Calendar", t: "Design review starts in 10 minutes" },
]

// Click a note to swipe it away, or "Clear all". They come back after a moment so the demo loops.
export function Notifications() {
  const [gone, setGone] = useState<number[]>([])
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined)
  const box = useRef<HTMLDivElement>(null)
  const empty = gone.length === NOTES.length

  useEffect(() => {
    if (!empty) return
    const r = box.current?.getBoundingClientRect()
    if (r) burst(r.left + r.width / 2, r.top + r.height / 2, 10)
    timer.current = setTimeout(() => setGone([]), 2400)
    return () => clearTimeout(timer.current)
  }, [empty])

  const clearAll = () => {
    NOTES.forEach((_, i) => setTimeout(() => setGone((g) => (g.includes(i) ? g : [...g, i])), i * 70))
  }

  return (
    <div className="ui">
      <div className="ui__head">
        <span>Notifications</span>
        <button type="button" className="ui__clear" onClick={clearAll} disabled={empty}>Clear all</button>
      </div>
      <div className="notes" data-empty={empty} ref={box}>
        {NOTES.map((n, i) => (
          <div className="note-wrap" data-gone={gone.includes(i)} key={n.b}>
            <div>
              <button type="button" className="note" tabIndex={gone.includes(i) ? -1 : 0} onClick={() => setGone((g) => [...g, i])}>
                <span className="note__ic" />
                <span className="note__txt"><b>{n.b}</b><span>{n.t}</span></span>
              </button>
            </div>
          </div>
        ))}
        <p className="notes__empty" aria-live="polite">
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3 8.5 3.2 3.2L13 4.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
          {empty ? "You're all caught up" : ""}
        </p>
      </div>
    </div>
  )
}

// Exact while held (no transition), keyboard with arrows.
export function Slider({ initial, label, className }: { initial: number; label: string; className: string }) {
  const [v, setV] = useState(initial)
  const ref = useRef<HTMLDivElement>(null)
  const dragging = useRef(false)
  const set = (clientX: number) => {
    const r = ref.current!.getBoundingClientRect()
    setV(Math.round(Math.min(1, Math.max(0, (clientX - r.left) / r.width)) * 100))
  }
  return (
    <div
      ref={ref}
      className={`${className} drag`}
      role="slider"
      tabIndex={0}
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={v}
      onPointerDown={(e) => {
        dragging.current = true
        try {
          e.currentTarget.setPointerCapture(e.pointerId)
        } catch {
          /* the page's demo cursor sends fake pointers */
        }
        set(e.clientX)
      }}
      onPointerMove={(e) => dragging.current && set(e.clientX)}
      onPointerUp={() => (dragging.current = false)}
      onPointerCancel={() => (dragging.current = false)}
      onKeyDown={(e) => {
        const d = e.key === "ArrowRight" || e.key === "ArrowUp" ? 5 : e.key === "ArrowLeft" || e.key === "ArrowDown" ? -5 : 0
        if (d) {
          e.preventDefault()
          setV((x) => Math.min(100, Math.max(0, x + d)))
        }
      }}
    >
      <b style={{ width: `${v}%` }} />
    </div>
  )
}

export function ControlCenter() {
  const [on, setOn] = useState([true, false, true, false])
  const names = ["Wi-Fi", "Bluetooth", "Focus", "Night light"]
  return (
    <div className="ui">
      <div className="tiles">
        {on.map((v, i) => (
          <button
            type="button"
            key={names[i]}
            className={`tl${v ? " on" : ""}`}
            aria-pressed={v}
            aria-label={names[i]}
            onClick={() => setOn((s) => s.map((x, j) => (j === i ? !x : x)))}
          >
            <i />
          </button>
        ))}
      </div>
      <Slider className="slider" initial={62} label="Volume" />
      <Slider className="slider" initial={38} label="Brightness" />
    </div>
  )
}

// Sliding thumb: one element travels between options instead of each option swapping its own background.
export function Seg({ options, initial = 0, label }: { options: string[]; initial?: number; label: string }) {
  const [i, setI] = useState(initial)
  return (
    <span className="seg" role="radiogroup" aria-label={label} style={{ "--n": options.length, "--i": i } as React.CSSProperties}>
      <i className="seg__thumb" />
      {options.map((o, j) => (
        <button type="button" role="radio" aria-checked={i === j} key={o} data-on={i === j} onClick={() => setI(j)}>
          {o}
        </button>
      ))}
    </span>
  )
}

export function Monitors() {
  const [active, setActive] = useState(0)
  return (
    <div className="tile__media monitors">
      {[0, 1].map((n) => (
        <button
          type="button"
          key={n}
          className={`mon${n === 1 ? " mon--s" : ""}`}
          data-active={active === n}
          aria-pressed={active === n}
          aria-label={`Put the bar on monitor ${n + 1}`}
          onClick={() => setActive(n)}
        />
      ))}
    </div>
  )
}

const TRAY = [
  { n: "Wi-Fi", items: ["Home network", "Turn Wi-Fi off", "Open settings"] },
  { n: "Dropbox", items: ["Open folder", "Pause syncing", "Quit"] },
  { n: "Spotify", items: ["Play / Pause", "Next song", "Quit"] },
  { n: "Slack", items: ["Open", "Set status", "Quit"] },
  { n: "Docker", items: ["Dashboard", "Restart", "Quit"] },
]

// Click a tray icon and its menu drops right under it, like in the bar.
export function TrayRow() {
  const [open, setOpen] = useState<number | null>(null)
  const ref = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (open === null) return
    const away = (e: PointerEvent) => !ref.current?.contains(e.target as Node) && setOpen(null)
    const esc = (e: KeyboardEvent) => e.key === "Escape" && setOpen(null)
    document.addEventListener("pointerdown", away)
    document.addEventListener("keydown", esc)
    return () => {
      document.removeEventListener("pointerdown", away)
      document.removeEventListener("keydown", esc)
    }
  }, [open])
  return (
    <div className="tray" ref={ref}>
      <div className="trayrow">
        {TRAY.map((t, i) => (
          <button
            type="button"
            key={t.n}
            className={`tray__icon${i % 3 === 2 ? " white" : ""}`}
            aria-label={t.n}
            aria-expanded={open === i}
            data-open={open === i}
            onClick={() => setOpen(open === i ? null : i)}
          />
        ))}
        <span>22:00</span>
      </div>
      {open !== null && (
        <div className="tray__menu" key={open} style={{ left: 16 + open * 36 }} role="menu">
          <b>{TRAY[open]?.n}</b>
          {TRAY[open]?.items.map((it) => (
            <button type="button" role="menuitem" key={it} onClick={() => setOpen(null)}>{it}</button>
          ))}
        </div>
      )}
    </div>
  )
}

// The SDK example, alive: the item in the bar counts when you click it, like the code says.
export function MiniBar() {
  const [n, setN] = useState(0)
  return (
    <div className="minibar">
      <span className="minibar__name">Explorer</span>
      <span className="minibar__sp" />
      <button
        type="button"
        className="minibar__item"
        aria-label={`counter, ${n}. Click to add one`}
        onClick={(e) => {
          setN((x) => x + 1)
          if ((n + 1) % 5 === 0) {
            const r = e.currentTarget.getBoundingClientRect()
            burst(r.left + r.width / 2, r.top + r.height / 2, 10)
          }
        }}
      >
        <span key={n} className="roll">{n}</span>
      </button>
      <span className="minibar__time">22:00</span>
    </div>
  )
}

// Word by word the statement lights up as you scroll into it.
export function Scrub({ text }: { text: string }) {
  const ref = useRef<HTMLParagraphElement>(null)
  const words = text.split(" ")
  useEffect(() => {
    const el = ref.current!
    const spans = Array.from(el.querySelectorAll<HTMLElement>("span"))
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      spans.forEach((s) => s.classList.add("lit"))
      return
    }
    let frame = 0
    const update = () => {
      frame = 0
      const r = el.getBoundingClientRect()
      const vh = window.innerHeight
      const p = Math.min(1, Math.max(0, (vh * 0.85 - r.top) / (vh * 0.45 + r.height * 0.5)))
      const lit = Math.round(p * spans.length)
      spans.forEach((s, i) => s.classList.toggle("lit", i < lit))
    }
    const on = () => {
      if (!frame) frame = requestAnimationFrame(update)
    }
    update()
    window.addEventListener("scroll", on, { passive: true })
    window.addEventListener("resize", on)
    return () => {
      window.removeEventListener("scroll", on)
      window.removeEventListener("resize", on)
      cancelAnimationFrame(frame)
    }
  }, [])
  return (
    <p ref={ref} className="statement text-cut scrub">
      {words.map((w, i) => (
        <span key={i}>{w} </span>
      ))}
    </p>
  )
}

export function DownloadLink({ white = false }: { white?: boolean }) {
  return (
    <a
      href={DOWNLOAD_URL}
      className={`btn btn--lg ${white ? "btn--white" : "btn--primary"}`}
      onPointerDown={(e) => {
        const r = e.currentTarget.getBoundingClientRect()
        burst(r.left + r.width / 2, r.top + r.height / 2, 14)
      }}
    >
      <WindowsGlyph />
      Download for Windows
    </a>
  )
}

export function SoonButton() {
  const [t, setT] = useState(0)
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined)
  useEffect(() => () => clearTimeout(timer.current), [])
  return (
    <button
      type="button"
      key={t}
      className="btn btn--lg btn--soon"
      data-shake={t > 0}
      onClick={() => {
        clearTimeout(timer.current)
        setT((x) => x + 1)
        timer.current = setTimeout(() => setT(0), 1800)
      }}
    >
      {t > 0 ? "Soon. Promise." : "Coming soon"}
    </button>
  )
}
