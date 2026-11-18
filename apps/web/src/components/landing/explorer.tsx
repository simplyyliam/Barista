"use client"

import { AnimatePresence, motion, useReducedMotion } from "motion/react"
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react"

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

const PILL_H = 56
const spring = { type: "spring", stiffness: 720, damping: 50, mass: 0.8 } as const

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
  // `open` is the expanded pill (none at the start). `shown` is the feature whose media is on screen.
  const [open, setOpen] = useState<number | null>(null)
  const [shown, setShown] = useState(0)
  // Which edge the text comes in from: picking a feature above the open one comes from the bottom,
  // below comes from the top. The first feature always counts as "above".
  const [origin, setOrigin] = useState<"top" | "bottom">("top")
  const dir = useRef(1)
  const t = reduce ? { duration: 0 } : spring

  // Closed pills hug their label, so measure each one: width and height are then animated for real
  // (not squashed with a transform), which keeps the corners round the whole way.
  const labels = useRef<(HTMLButtonElement | null)[]>([])
  const [widths, setWidths] = useState<number[]>([])
  useLayoutEffect(() => {
    const measure = () => setWidths(labels.current.map((b) => b?.offsetWidth ?? 0))
    measure()
    void document.fonts?.ready.then(measure)
  }, [])

  const go = useCallback(
    (next: number) => {
      const n = (next + FEATURES.length) % FEATURES.length
      if (n === open) {
        setOpen(null)
        return
      }
      dir.current = n > shown ? 1 : -1
      setOrigin(n === 0 || (open !== null && n < open) ? "bottom" : "top")
      setOpen(n)
      setShown(n)
    },
    [open, shown],
  )

  useEffect(() => {
    if (!modal || !onClose) return
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose()
    window.addEventListener("keydown", onKey)
    return () => window.removeEventListener("keydown", onKey)
  }, [modal, onClose])

  const f = FEATURES[shown] as Feature
  const d = dir.current

  return (
    <div
      className={modal ? "tour tour--modal" : "tour"}
      onKeyDown={(e) => {
        if (e.key === "ArrowDown") { e.preventDefault(); go(open === null ? 0 : shown + 1) }
        if (e.key === "ArrowUp") { e.preventDefault(); go(open === null ? FEATURES.length - 1 : shown - 1) }
      }}
    >
      {onClose && (
        <button type="button" className="tour__close" onClick={onClose} aria-label="Close tour">
          <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="m3 3 10 10M13 3 3 13" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /></svg>
        </button>
      )}

      <div className="tour__side">
        <div className="tour__nav" aria-label="Step through features">
          <button type="button" onClick={() => go(open === null ? FEATURES.length - 1 : shown - 1)} aria-label="Previous feature"><Chevron up /></button>
          <button type="button" onClick={() => go(open === null ? 0 : shown + 1)} aria-label="Next feature"><Chevron /></button>
        </div>
        <div className="tour__list">
          <ul className="tour__pills">
            {FEATURES.map((x, i) => (
              <motion.li
                key={x.id}
                className="tour__pill"
                initial={false}
                animate={{ height: i === open ? "auto" : PILL_H, width: i === open ? "100%" : widths[i] || "auto" }}
                transition={t}
                style={{ borderRadius: PILL_H / 2 }}
              >
                <button ref={(el) => { labels.current[i] = el }} type="button" className="tour__pillbtn" aria-expanded={i === open} onClick={() => go(i)}>
                  <motion.span className="tour__plus" animate={{ rotate: i === open ? 45 : 0 }} transition={t}><PlusIcon /></motion.span>
                  {x.label}
                </button>
                <AnimatePresence initial={false}>
                  {i === open && (
                    <motion.p
                      className="tour__text"
                      initial={{ opacity: 0, y: reduce ? 0 : origin === "bottom" ? 12 : -12 }}
                      animate={{ opacity: 1, y: 0, transition: reduce ? { duration: 0 } : { ...spring, opacity: { duration: 0.2, delay: 0.05 } } }}
                      exit={{ opacity: 0, transition: { duration: 0.08 } }}
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
          if (Math.abs(info.offset.y) > 50) go(shown + (info.offset.y < 0 ? 1 : -1))
        }}
        style={{ touchAction: "pan-x" }}
      >
        <AnimatePresence mode="popLayout" initial={false} custom={d}>
          <motion.div
            key={f.id}
            className="tour__slot"
            initial={{ opacity: 0, y: reduce ? 0 : 20 * d, scale: reduce ? 1 : 1.02 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: reduce ? 0 : -20 * d, scale: reduce ? 1 : 0.98 }}
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
