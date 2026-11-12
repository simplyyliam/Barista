import { useEffect, useState } from "react"
import { commands } from "@/shared/bindings"

/** The picture of a notification (a screenshot, a photo). Loaded when the row shows up. */
export function NotificationImage({ id }: { id: number }) {
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
  // Wider than the text so it reaches nearly to the card's edge, like App.png.
  return (
    <img
      src={src}
      alt=""
      draggable={false}
      className="mt-2 -mx-2 max-h-56 w-[calc(100%+1rem)] max-w-none rounded-panel-child object-cover"
    />
  )
}
