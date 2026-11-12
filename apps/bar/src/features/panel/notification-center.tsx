import { useEffect } from "react"
import { cn } from "cn"
import { commands } from "@/shared/bindings"
import { useNotifications } from "@/shared/stores/notifications"
import { Icon, LucideBell } from "@barista/ui/icons"
import { AppIcon } from "./app-icon"
import { NotificationRow } from "./notification-row"

/**
 * How tall the list is: about what one notification with a picture takes. A picture notification
 * fills it (you scroll for the next one); text notifications are packed in as many as fit. The
 * whole card stack must also fit the screen (the calendar, Focus pills, gaps and the bar take about
 * 600 px), so a short screen gets a shorter list.
 */
function listHeight() {
  return Math.min(300, Math.max(140, window.screen.availHeight - 600))
}

/** The black "Notifications" card of App.png. `active`: the panel is open (not just prebaked). */
export function NotificationCenter({ active }: { active: boolean }) {
  const notifications = useNotifications()
  const groups = notifications?.groups ?? []
  // One block per notification, newest first, each with its own app header like App.png (WhatsApp and
  // Discord are separate cards there), so a picture notification can be the only thing in view.
  const entries = groups
    .flatMap((group) => group.items.map((item) => ({ group, item })))
    .sort((a, b) => b.item.arrivedAt - a.item.arrivedAt || b.item.id - a.item.id)
  const unread = notifications?.unread ?? false
  const height = listHeight()

  // Opening the panel counts as reading, and so does something arriving while it is open. A hidden,
  // prebaked panel must not mark anything as read.
  useEffect(() => {
    if (active && unread) void commands.markNotificationsSeen()
  }, [active, unread])

  return (
    <section
      aria-label="Notifications"
      // 8 px at the bottom like App.png; text groups add their own space, pictures keep the 8.
      className="w-panel-w rounded-panel bg-panel p-panel-pad pb-panel-inset text-panel-fg"
    >
      <header className="flex items-center justify-between">
        <h2 className="text-panel-title font-normal">Notifications</h2>
        <div className="flex items-center gap-4">
          <button
            type="button"
            aria-label="Notification settings"
            onClick={() => void commands.openNotificationSettings()}
            className="grid size-6 place-items-center rounded-full outline-none focus-visible:ring-2 focus-visible:ring-panel-fg/50"
          >
            <Icon icon={LucideBell} size={16} />
          </button>
          <button
            type="button"
            disabled={entries.length === 0}
            onClick={() => void commands.clearNotifications()}
            className="text-panel-title outline-none focus-visible:ring-2 focus-visible:ring-panel-fg/50 disabled:text-panel-fg-muted"
          >
            Clear all
          </button>
        </div>
      </header>

      {notifications && notifications.access !== "allowed" ? (
        <AccessOff />
      ) : entries.length === 0 ? (
        <p className="pt-10 pb-8 text-center text-panel-small text-panel-fg-muted">No notifications</p>
      ) : (
        <div
          style={{ maxHeight: height }}
          // Full card width with its own padding: a scrolling box clips whatever sticks out of it, and
          // the pictures stick out 8 px past the text. Snapping lands a picture notification exactly
          // in the window, nothing else beside it.
          className="-mx-panel-pad mt-4 flex snap-y snap-proximity flex-col gap-panel-gap overflow-y-auto px-panel-pad"
        >
          {entries.map(({ group, item }, index) => (
            <div
              key={item.id}
              // A picture notification fills the window, so it is the only thing in view.
              style={item.hasImage ? { minHeight: height } : undefined}
              className={cn(
                "flex snap-start flex-col gap-2",
                index === entries.length - 1 && !item.hasImage && "pb-2",
              )}
            >
              <div className="flex items-center gap-2">
                <AppIcon appId={group.appId} name={group.appName} />
                <h3 className="truncate text-panel-title font-normal">{group.appName}</h3>
              </div>
              <NotificationRow item={item} appName={group.appName} />
            </div>
          ))}
        </div>
      )}
    </section>
  )
}

/** Windows says no (or has not said yes): send the user to the one switch that fixes it. */
function AccessOff() {
  return (
    <div className="mt-4 flex flex-col items-start gap-3 py-2 pb-4">
      <p className="text-panel-small text-panel-fg-muted">
        Turn on notification access for Barista to show your notifications here.
      </p>
      <button
        type="button"
        onClick={() => void commands.openNotificationSettings()}
        className="rounded-full bg-panel-fg px-4 py-1.5 text-panel-small font-medium text-panel outline-none focus-visible:ring-2 focus-visible:ring-panel-fg/50"
      >
        Open settings
      </button>
    </div>
  )
}
