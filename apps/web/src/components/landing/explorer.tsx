"use client"

import { AnimatePresence, motion, useReducedMotion } from "motion/react"
import { useCallback, useEffect, useRef, useState } from "react"

// Prototype of the "highlights explorer" (v1.0 website). Built from the public pattern, no code copied.
// Put a real clip in `video` (and a `poster`) to replace the placeholder slot.
type Feature = { id: string; label: string; text: string; video?: string; poster?: string }

export const FEATURES: Feature[] = [
  { id: "tray", label: "Tray icons", text: "Every app from the Windows tray lives in the bar. Click one and its own menu opens right under it." },
  { id: "notifications", label: "Notifications", text: "Read, clear and open notifications from a panel under the clock." },
  { id: "control", label: "Control center", text: "Wi-Fi, Bluetooth, brightness, volume and now playing in one card." },
  { id: "monitors", label: "Every monitor", text: "Pick the screen for the bar. It follows your display scale and keeps its space reserved." },
  { id: "windows", label: "Windows mode", text: "Everything Barista replaces has a Windows mode. Flip it and the bar steps aside." },
  { id: "dev", label: "For developers", text: "Add a status item with a few lines of the SDK. Send a menu or open your own popover." },
]

const spring = { type: "spring", stiffness: 380, damping: 36, mass: 0.9 } as const

function Chevron({ up }: { up?: boolean }) {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" style={{ transform: up ? "rotate(180deg)" : undefined }}>
      <path d="m3.5 6 4.5 4.5L12.5 6" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

function PlusIcon() {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
      <circle cx="8" cy="8" r="7" fill="none" stroke="currentColor" strokeWidth="1.4" />
      <path d="M8 4.8v6.4M4.8 8h6.4" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  )
}

export function Explorer({ onClose, modal = false }: { onClose?: () => void; modal?: boolean }) {
  const reduce = useReducedMotion()
  const [active, setActive] = useState(0)
  const dir = useRef(1)
  const t = reduce ? { duration: 0 } : spring

  const go = useCallback((next: number) => {
    const n = (next + FEATURES.length) % FEATURES.length
    setActive((cur) => {
      dir.current = n >= cur ? 1 : -1
      return n
    })
  }, [])

  useEffect(() => {
    if (!modal || !onClose) return
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose()
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [modal, onClose])

  const f = FEATURES[active] as Feature
  const d = dir.current

  return (
    <div
      className={modal ? "tour tour--modal" : "tour"}
      onKeyDown={(e) => {
        if (e.key === "ArrowDown") { e.preventDefault(); go(active + 1) }
        if (e.key === "ArrowUp") { e.preventDefault(); go(active - 1) }
      }}
    >
      {onClose && (
        <button type="button" className="tour__close" onClick={onClose} aria-label="Close tour">
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="m3 3 10 10M13 3 3 13" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /></svg>
        </button>
      )}

      <div className="tour__side">
        <div className="tour__nav" aria-label="Step through features">
          <button type="button" onClick={() => go(active - 1)} aria-label="Previous feature"><Chevron up /></button>
          <button type="button" onClick={() => go(active + 1)} aria-label="Next feature"><Chevron /></button>
        </div>
        <div className="tour__list">
          <ul className="tour__pills">
            {FEATURES.map((x, i) => (
              <motion.li key={x.id} layout transition={t} className="tour__pill" data-active={i === active} style={{ borderRadius: 32 }}>
                <button type="button" className="tour__pillbtn" aria-expanded={i === active} onClick={() => go(i)}>
                  <motion.span className="tour__plus" animate={{ rotate: i === active ? 45 : 0 }} transition={t}><PlusIcon /></motion.span>
                  {x.label}
                </button>
                <AnimatePresence initial={false}>
                  {i === active && (
                    <motion.p
                      className="tour__text"
                      initial={{ height: 0, opacity: 0 }}
                      animate={{ height: "auto", opacity: 1 }}
                      exit={{ height: 0, opacity: 0 }}
                      transition={t}
                    >
                      {x.text}
                    </motion.p>
                  )}
                </AnimatePresence>
              </motion.li>
            ))}
          </ul>
        </div>
      </div>

      <motion.div
        className="tour__media"
        onPanEnd={(_, info) => {
          if (Math.abs(info.offset.y) > 50) go(active + (info.offset.y < 0 ? 1 : -1))
        }}
        style={{ touchAction: "pan-x" }}
      >
        <AnimatePresence mode="popLayout" initial={false} custom={d}>
          <motion.div
            key={f.id}
            className="tour__slot"
            initial={{ opacity: 0, y: reduce ? 0 : 28 * d, scale: reduce ? 1 : 1.03 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: reduce ? 0 : -28 * d, scale: reduce ? 1 : 0.98 }}
            transition={reduce ? { duration: 0 } : { ...spring, opacity: { duration: 0.25 } }}
          >
            {f.video ? (
              <video src={f.video} poster={f.poster} muted loop playsInline autoPlay preload="metadata" />
            ) : (
              <div className="tour__ph">
                <span className="tour__play" aria-hidden="true"><svg viewBox="0 0 16 16" width="18" height="18"><path d="M5 3.2v9.6L13 8z" fill="currentColor" /></svg></span>
                <strong>{f.label}</strong>
                <span>Product video goes here</span>
              </div>
            )}
          </motion.div>
        </AnimatePresence>
      </motion.div>
    </div>
  )
}

export function TourSection() {
  const [open, setOpen] = useState(false)
  const opener = useRef<HTMLButtonElement>(null)
  const close = useCallback(() => {
    setOpen(false)
    opener.current?.focus()
  }, [])

  useEffect(() => {
    if (!open) return
    const prev = document.body.style.overflow
    document.body.style.overflow = "hidden"
    return () => { document.body.style.overflow = prev }
  }, [open])

  return (
    <section className="section" id="tour">
      <div className="wrap">
        <div className="stack center" style={{ textAlign: "center" }}>
          <p className="eyebrow" data-reveal>Product tour</p>
          <h2 className="h2" data-reveal data-delay="80">See it at work.</h2>
          <p className="lead" style={{ maxWidth: 620 }} data-reveal data-delay="160">
            Pick a feature on the left. The film on the right follows.
          </p>
          <button ref={opener} type="button" className="btn btn--lg btn--primary" onClick={() => setOpen(true)} data-reveal data-delay="240">
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M5 3.2v9.6L13 8z" /></svg>
            Watch the film
          </button>
        </div>
        <div style={{ marginTop: 60 }} data-reveal>
          <Explorer />
        </div>
      </div>
      <AnimatePresence>
        {open && (
          <motion.div
            className="tour__overlay"
            role="dialog"
            aria-modal="true"
            aria-label="Barista product tour"
            initial={{ opacity: 0, scale: 0.985 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.985 }}
            transition={{ duration: 0.25 }}
          >
            <Explorer modal onClose={close} />
          </motion.div>
        )}
      </AnimatePresence>
    </section>
  )
}
