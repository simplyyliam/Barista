import { useEffect, useState } from "react"
import { CalendarCard } from "./calendar-card"
import { FocusRow } from "./focus-row"
import { NotificationCenter } from "./notification-center"

/**
 * The `notifications` route: notification card, calendar card and the Focus pills (App.png).
 * `active` is false while the window is hidden: everything stays mounted and loaded, but it does
 * not count as "read".
 */
export function NotificationsPanel({ active }: { active: boolean }) {
  // The calendar starts at today every time, so it is reset while the window is hidden (after a
  // close), never as the panel opens: a remount on open would be a second paint.
  const [calendarKey, setCalendarKey] = useState(0)
  useEffect(() => {
    if (!active) setCalendarKey((key) => key + 1)
  }, [active])

  return (
    <div className="flex w-panel-w flex-col gap-4">
      <NotificationCenter active={active} />
      <CalendarCard key={calendarKey} />
      <FocusRow />
    </div>
  )
}
