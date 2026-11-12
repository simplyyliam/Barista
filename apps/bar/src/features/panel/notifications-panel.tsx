import { CalendarCard } from "./calendar-card"
import { FocusRow } from "./focus-row"
import { NotificationCenter } from "./notification-center"

/**
 * The `notifications` route: notification card, calendar card and the Focus pills (App.png).
 * `active` is false while the window is hidden: everything stays mounted and loaded, but it does
 * not count as "read".
 */
export function NotificationsPanel({ active }: { active: boolean }) {
  return (
    <div className="flex w-panel-w flex-col gap-4">
      <NotificationCenter active={active} />
      {/* New every open, so the month and the picked day start at today again. */}
      <CalendarCard key={active ? "open" : "closed"} />
      <FocusRow />
    </div>
  )
}
