import { useEffect } from "react"
import { cn } from "cn"
import { commands } from "@/shared/bindings"
import { useNotifications } from "@/shared/stores/notifications"
import { Icon, LucideBell } from "@barista/ui/icons"
import { AppIcon } from "./app-icon"
import { NotificationRow } from "./notification-row"

/**
 * How tall the list may get. The calendar card, Focus pills, gaps and the bar take about 600 px, and
 * the whole stack must fit the screen, so a short screen gets a shorter (scrolling) list.
 */
function listMaxHeight() {
  return Math.min(420, Math.max(140, window.screen.availHeight - 600))
}

/** The black "Notifications" card of App.png. */
export function NotificationCenter() {
  const notifications = useNotifications()
  const groups = notifications?.groups ?? []
  const unread = notifications?.unread ?? false
  // A picture at the bottom sits 8 px from the card edge, like App.png, not 16.
  const endsWithPicture = groups.at(-1)?.items.at(-1)?.hasImage === true

  // Opening the panel counts as reading (the first snapshot arrives with `unread` true), and so
  // does something arriving while it is open.
  useEffect(() => {
    if (unread) void commands.markNotificationsSeen()
  }, [unread])

  return (
    <section
      aria-label="Notifications"
      className={cn(
        "w-panel-w rounded-panel bg-panel p-panel-pad text-panel-fg",
        endsWithPicture && "pb-panel-inset",
      )}
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
            disabled={groups.length === 0}
            onClick={() => void commands.clearNotifications()}
            className="text-panel-title outline-none focus-visible:ring-2 focus-visible:ring-panel-fg/50 disabled:text-panel-fg-muted"
          >
            Clear all
          </button>
        </div>
      </header>

      {notifications && notifications.access !== "allowed" ? (
        <AccessOff />
      ) : groups.length === 0 ? (
        <p className="py-10 text-center text-panel-small text-panel-fg-muted">No notifications</p>
      ) : (
        <div
          style={{ maxHeight: listMaxHeight() }}
          // Full card width with its own padding: a scrolling box clips whatever sticks out of it, and
          // the pictures stick out 8 px past the text.
          className="-mx-panel-pad mt-4 flex flex-col gap-panel-gap overflow-y-auto px-panel-pad"
        >
          {groups.map((group) => (
            <div key={group.appId} className="flex flex-col gap-2">
              <div className="flex items-center gap-2">
                <AppIcon appId={group.appId} name={group.appName} />
                <h3 className="truncate text-panel-title font-normal">{group.appName}</h3>
              </div>
              {group.items.map((item) => (
                <NotificationRow key={item.id} item={item} appName={group.appName} />
              ))}
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
    <div className="mt-4 flex flex-col items-start gap-3 py-2">
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
