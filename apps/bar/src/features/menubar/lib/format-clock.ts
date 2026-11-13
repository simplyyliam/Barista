/**
 * The bar's clock text, like macOS: "Thu Nov 12" and "1:18 PM" (or "13:18" on a 24 hour clock).
 * Two parts, because the bar puts a gap (two spaces wide) between them. Always English: the
 * clock service only tells us 12 or 24 hour.
 */
export function formatClock(now: Date, hour24: boolean) {
  const weekday = now.toLocaleDateString("en-US", { weekday: "short" })
  const monthDay = now.toLocaleDateString("en-US", { month: "short", day: "numeric" })
  const time = now.toLocaleTimeString("en-US", {
    hour: hour24 ? "2-digit" : "numeric",
    minute: "2-digit",
    hourCycle: hour24 ? "h23" : "h12",
  })
  return { date: `${weekday} ${monthDay}`, time }
}
