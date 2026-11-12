import { CalendarCard } from "./calendar-card"
import { FocusRow } from "./focus-row"
import { NotificationCenter } from "./notification-center"

/** The `notifications` route: notification card, calendar card and the Focus pills (App.png). */
export function NotificationsPanel() {
  return (
    <div className="flex w-panel-w flex-col gap-4">
      <NotificationCenter />
      <CalendarCard />
      <FocusRow />
    </div>
  )
}
