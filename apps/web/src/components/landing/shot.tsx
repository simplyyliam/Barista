// Screenshot placeholder. Replace the <div> with an <img> when the real screenshots exist.
export function Shot({
  label,
  size,
  ratio = "16/10",
  className = "",
}: {
  label: string
  size: string
  ratio?: string
  className?: string
}) {
  return (
    <div
      className={`shot ${className}`}
      style={{ "--ratio": ratio } as React.CSSProperties}
      role="img"
      aria-label={`${label} (screenshot coming soon)`}
    >
      <b>{label}</b>
      <span>{size}</span>
    </div>
  )
}
