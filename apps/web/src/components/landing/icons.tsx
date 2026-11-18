"use client"

import { burst } from "./demo"

export function WindowsGlyph() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3 5.5 10.5 4.5v7H3zM11.5 4.4 21 3v8.5h-9.5zM3 12.5h7.5v7L3 18.5zM11.5 12.5H21V21l-9.5-1.4z" />
    </svg>
  )
}

export function Check() {
  return (
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path d="m3 8.5 3.2 3.2L13 4.5" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

export function Plus() {
  return (
    <svg className="faq__icon" viewBox="0 0 16 16" aria-hidden="true">
      <path d="M8 2v12M2 8h12" strokeLinecap="round" />
    </svg>
  )
}

// The mascot: a coffee bean with two eyes. Eyes follow the cursor (see reveal.tsx), it blinks now
// and then, and a click makes it hop.
export function Bean({ className = "", style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <span
      className={`bean ${className}`}
      style={style}
      aria-hidden="true"
      onClick={(e) => {
        const svg = e.currentTarget.querySelector("svg")!
        svg.classList.remove("hop")
        void svg.getBoundingClientRect()
        svg.classList.add("hop")
        const r = e.currentTarget.getBoundingClientRect()
        burst(r.left + r.width / 2, r.top + r.height * 0.3, 10)
      }}
    >
      <svg viewBox="0 0 140 160">
        <ellipse cx="70" cy="82" rx="52" ry="68" transform="rotate(-14 70 82)" fill="var(--bean)" />
        <path d="M70 18c-22 28 22 52-2 78s10 40-8 62" transform="rotate(-14 70 82)" fill="none" stroke="#fff" strokeOpacity=".28" strokeWidth="5" strokeLinecap="round" />
        <g className="eyes">
          <rect x="38" y="62" width="24" height="38" rx="12" fill="#fff" />
          <rect x="78" y="62" width="24" height="38" rx="12" fill="#fff" />
          <circle className="pupil" cx="50" cy="84" r="6.5" fill="#0a0a0a" />
          <circle className="pupil" cx="90" cy="84" r="6.5" fill="#0a0a0a" />
        </g>
      </svg>
    </span>
  )
}

export function BeanGlyph() {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <ellipse cx="12" cy="12" rx="6.2" ry="9" transform="rotate(-20 12 12)" fill="#fff" />
      <path d="M11 4.5c-3 3.5 3 6 0 9" transform="rotate(-20 12 12)" fill="none" stroke="var(--bean)" strokeWidth="1.6" strokeLinecap="round" />
    </svg>
  )
}
