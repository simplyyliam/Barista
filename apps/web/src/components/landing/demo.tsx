"use client"

import { useEffect, useRef, useState } from "react"

// A ghost cursor that plays a short demo on a piece of the page, so nobody has to guess it is
// interactive. It uses real DOM events (click, pointer), so the components know nothing about it.
// The first real pointer press or key inside the piece stops it for good and the hint fades.

export type Ghost = {
  q: (sel: string, i?: number) => HTMLElement | null
  tap: (el: HTMLElement | null) => Promise<void>
  drag: (el: HTMLElement | null, to: number) => Promise<void>
  wait: (ms: number) => Promise<void>
}

const SCRIPTS = {
  tray: async (g) => {
    await g.tap(g.q(".tray__icon", 1))
    await g.wait(1500)
    await g.tap(g.q(".tray__icon", 3))
    await g.wait(1500)
    await g.tap(g.q(".tray__icon", 3))
  },
  monitors: async (g) => {
    await g.tap(g.q(".mon", 1))
    await g.wait(1300)
    await g.tap(g.q(".mon", 0))
  },
  settings: async (g) => {
    await g.drag(g.q(".set__slider", 0), 0.85)
    await g.wait(250)
    await g.drag(g.q(".set__slider", 1), 0.3)
    await g.wait(400)
    for (let round = 0; round < 2; round++) {
      await g.tap(g.q('.seg button[data-on="false"]'))
      await g.wait(900)
    }
  },
  switches: async (g) => {
    for (let round = 0; round < 2; round++) {
      for (let i = 0; i < 3; i++) {
        await g.tap(g.q(`.set__row:nth-child(${i + 1}) .seg button[data-on="false"]`))
        await g.wait(400)
      }
      await g.wait(600)
    }
  },
} satisfies Record<string, (g: Ghost) => Promise<void>>

export type DemoName = keyof typeof SCRIPTS

const STOP = Symbol("stop")

export function Demo({
  script,
  hint,
  delay = 0,
  className = "",
  role,
  label,
  children,
}: {
  script: DemoName
  hint?: string
  delay?: number
  className?: string
  role?: string
  label?: string
  children: React.ReactNode
}) {
  const root = useRef<HTMLDivElement>(null)
  const cur = useRef<HTMLSpanElement>(null)
  const [done, setDone] = useState(false)

  useEffect(() => {
    const el = root.current!
    const c = cur.current!
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return

    let stopped = false
    let visible = false
    let running = false
    let placed = false
    const timers = new Set<ReturnType<typeof setTimeout>>()

    const wait = (ms: number) =>
      new Promise<void>((res, rej) => {
        const t = setTimeout(() => {
          timers.delete(t)
          if (stopped || !visible) rej(STOP)
          else res()
        }, ms)
        timers.add(t)
      })

    const spot = (t: HTMLElement, fx = 0.5, fy = 0.5) => {
      const r = el.getBoundingClientRect()
      const b = t.getBoundingClientRect()
      return { x: b.left - r.left + b.width * fx, y: b.top - r.top + b.height * fy }
    }
    const put = (x: number, y: number) => {
      c.style.transform = `translate(${x}px,${y}px)`
    }
    const move = async (t: HTMLElement, fx?: number, fy?: number) => {
      const { x, y } = spot(t, fx, fy)
      if (!placed) {
        placed = true
        c.style.transition = "none"
        put(x + 70, y + 70)
        void c.offsetWidth
        c.style.transition = ""
        c.style.opacity = "1"
      }
      put(x, y)
      await wait(850)
    }

    const g: Ghost = {
      q: (sel, i = 0) => el.querySelectorAll<HTMLElement>(sel)[i] ?? null,
      wait,
      tap: async (t) => {
        if (!t) return
        await move(t, 0.5, 0.5)
        c.classList.add("tap")
        await wait(140)
        c.classList.remove("ripple")
        void c.offsetWidth
        c.classList.add("ripple")
        t.click()
        c.classList.remove("tap")
        await wait(260)
      },
      drag: async (t, to) => {
        if (!t) return
        const now = Number(t.getAttribute("aria-valuenow") ?? 50) / 100
        await move(t, now, 0.5)
        const r = t.getBoundingClientRect()
        const y = r.top + r.height / 2
        const fire = (type: string, frac: number) =>
          t.dispatchEvent(new PointerEvent(type, { bubbles: true, clientX: r.left + r.width * frac, clientY: y, pointerId: 99, pointerType: "mouse" }))
        fire("pointerdown", now)
        c.classList.add("tap", "lin")
        const end = spot(t, to, 0.5)
        put(end.x, end.y)
        const t0 = performance.now()
        const D = 800
        await new Promise<void>((res, rej) => {
          const step = () => {
            if (stopped || !visible) return rej(STOP)
            const k = Math.min(1, (performance.now() - t0) / D)
            fire("pointermove", now + (to - now) * k)
            if (k < 1) requestAnimationFrame(step)
            else res()
          }
          requestAnimationFrame(step)
        })
        fire("pointerup", to)
        c.classList.remove("tap", "lin")
        await wait(200)
      },
    }

    const run = async () => {
      if (running || stopped) return
      running = true
      try {
        await wait(delay)
        while (!stopped && visible) {
          await SCRIPTS[script](g)
          await wait(2600)
        }
      } catch {
        /* stopped or scrolled away */
      } finally {
        running = false
        c.style.opacity = "0"
        c.classList.remove("tap", "lin")
        placed = false
      }
    }

    const io = new IntersectionObserver(
      (entries) => {
        visible = entries[0]?.isIntersecting ?? false
        if (visible) void run()
      },
      { threshold: 0.6 },
    )
    io.observe(el)

    const stop = (e: Event) => {
      if (!e.isTrusted || stopped) return
      stopped = true
      setDone(true)
      c.style.opacity = "0"
      timers.forEach(clearTimeout)
    }
    el.addEventListener("pointerdown", stop, true)
    el.addEventListener("keydown", stop, true)
    return () => {
      stopped = true
      io.disconnect()
      timers.forEach(clearTimeout)
      el.removeEventListener("pointerdown", stop, true)
      el.removeEventListener("keydown", stop, true)
    }
  }, [script, delay])

  return (
    <>
      <div ref={root} className={`demo ${className}`} role={role} aria-label={label}>
        {children}
        <span ref={cur} className="ghost" aria-hidden="true">
          <svg viewBox="0 0 24 24"><path d="M2 2l7.5 19 2.7-8.2L21 10z" fill="#fff" stroke="#0a0a0a" strokeWidth="1.6" strokeLinejoin="round" /></svg>
        </span>
      </div>
      {hint && (
        <p className="hint" data-done={done} aria-hidden={done}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 3l5 15 2.4-6.2L19 9.5z" fill="currentColor" /></svg>
          {hint}
        </p>
      )}
    </>
  )
}

// Little pieces that pop out from a point: the "yes!" moment.
export function burst(x: number, y: number, n = 12) {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return
  const colors = ["#c4501a", "#0a0a0a", "#ffffff", "#e8935f"]
  for (let i = 0; i < n; i++) {
    const p = document.createElement("span")
    const s = 6 + Math.random() * 7
    Object.assign(p.style, {
      position: "fixed", left: `${x}px`, top: `${y}px`, width: `${s}px`, height: `${s * (Math.random() > 0.5 ? 1 : 1.6)}px`,
      borderRadius: "99px", background: colors[i % colors.length], pointerEvents: "none", zIndex: "100",
      boxShadow: "0 0 0 1px rgba(0,0,0,.08)",
    })
    document.body.appendChild(p)
    const a = (Math.PI * 2 * i) / n + Math.random() * 0.5
    const d = 50 + Math.random() * 70
    const anim = p.animate(
      [
        { transform: "translate(-50%,-50%) scale(.4) rotate(0)", opacity: 1 },
        { transform: `translate(calc(-50% + ${Math.cos(a) * d}px), calc(-50% + ${Math.sin(a) * d - 20}px)) scale(1) rotate(${Math.random() * 240}deg)`, opacity: 1, offset: 0.55 },
        { transform: `translate(calc(-50% + ${Math.cos(a) * d * 1.1}px), calc(-50% + ${Math.sin(a) * d + 40}px)) scale(.8) rotate(${Math.random() * 360}deg)`, opacity: 0 },
      ],
      { duration: 800 + Math.random() * 300, easing: "cubic-bezier(.2,.7,.3,1)" },
    )
    anim.onfinish = () => p.remove()
  }
}
