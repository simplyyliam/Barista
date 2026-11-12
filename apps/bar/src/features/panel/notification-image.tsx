import { useEffect, useState } from "react"
import { commands } from "@/shared/bindings"

/**
 * The picture of a notification (a screenshot, a photo), loaded when the row shows up. It sits
 * `--spacing-panel-inset` (8 px) from the card's edge like App.png, so it is wider than the text.
 * The wrapper is the same width as the image and clips it to the rounded corners, because a
 * button or a scrolling list would otherwise cut off anything that sticks out of it.
 * Its radius is the window radius minus that inset (ADR 0008).
 */
export function NotificationImage({ id, onOpen }: { id: number; onOpen: () => void }) {
  const [src, setSrc] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    void commands
      .getNotificationImage(id)
      .then((image) => !cancelled && setSrc(image))
      .catch(() => {})
    return () => {
      cancelled = true
    }
  }, [id])

  if (!src) return null
  return (
    <button
      type="button"
      aria-label="Open notification"
      onClick={onOpen}
      className="mt-2 -mx-2 block w-[calc(100%+1rem)] cursor-default overflow-hidden rounded-panel-child outline-none focus-visible:ring-2 focus-visible:ring-panel-fg/50"
    >
      <img src={src} alt="" draggable={false} className="block max-h-56 w-full object-cover" />
    </button>
  )
}
