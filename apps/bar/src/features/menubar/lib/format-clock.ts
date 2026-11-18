import type { ClockPrefs_Serialize } from "@/shared/bindings"

/**
 * The bar's clock text, like macOS: "Thu Nov 12" and "1:18 PM" (or "13:18" on a 24 hour clock).
 * Two parts, because the bar puts a gap (two spaces wide) between them. Always English: the
 * clock service only tells us 12 or 24 hour. `prefs` are the user's choices (settings window):
 * hour format (auto = what Windows uses), seconds, weekday, date.
 */
export function formatClock(now: Date, windowsHour24: boolean, prefs?: ClockPrefs_Serialize) {
  const hour24 = prefs?.hourFormat === "h24" ? true : prefs?.hourFormat === "h12" ? false : windowsHour24
  const parts: string[] = []
  if (prefs?.weekday ?? true) parts.push(now.toLocaleDateString("en-US", { weekday: "short" }))
  if (prefs?.date ?? true) parts.push(now.toLocaleDateString("en-US", { month: "short", day: "numeric" }))
  const time = now.toLocaleTimeString("en-US", {
    hour: hour24 ? "2-digit" : "numeric",
    minute: "2-digit",
    second: prefs?.seconds ? "2-digit" : undefined,
    hourCycle: hour24 ? "h23" : "h12",
  })
  return { date: parts.join(" "), time }
}
