import { commands, type Item } from "@/shared/bindings"
import { Icon, LucideX } from "@barista/ui/icons"
import { NotificationImage } from "./notification-image"

type NotificationRowProps = {
  item: Item
  /** The app's name: a title that only repeats it is left out. */
  appName: string
}

/** One notification: title, lines, optional small line and picture. Click opens the app. */
export function NotificationRow({ item, appName }: NotificationRowProps) {
  const showTitle = item.title !== "" && item.title !== appName
  return (
    <div className="group/row relative">
      <button
        type="button"
        onClick={() => void commands.openNotification(item.id)}
        className="block w-full cursor-default rounded-lg text-left outline-none focus-visible:ring-2 focus-visible:ring-panel-fg/50"
      >
        {showTitle && <p className="pr-6 text-[13px] leading-[17px] text-panel-fg">{item.title}</p>}
        {item.lines.map((line, i) => (
          <p key={i} className="pr-6 text-panel-small leading-[14px] text-panel-fg-muted">
            {line}
          </p>
        ))}
        {item.attribution && (
          <p className="pr-6 text-panel-small leading-[14px] text-panel-fg-muted">{item.attribution}</p>
        )}
        {item.hasImage && <NotificationImage id={item.id} />}
      </button>
      <button
        type="button"
        aria-label={`Dismiss notification from ${appName}`}
        onClick={() => void commands.dismissNotification(item.id)}
        className="absolute top-0 right-0 grid size-5 place-items-center rounded-full bg-panel-separator text-panel-fg opacity-0 transition-opacity outline-none group-hover/row:opacity-100 focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-panel-fg/50"
      >
        <Icon icon={LucideX} size={12} />
      </button>
    </div>
  )
}
