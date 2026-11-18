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

export function AppIcon({ large = false }: { large?: boolean }) {
  return <span className={large ? "appicon appicon--lg" : "appicon"} aria-hidden="true" />
}
