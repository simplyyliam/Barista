import { useEffect, useState } from "react"
import { commands } from "@/shared/bindings"

// One request per app for the whole session; Rust remembers them too.
const icons = new Map<string, Promise<string | null>>()

function loadIcon(appId: string) {
  let icon = icons.get(appId)
  if (!icon) {
    icon = commands.getNotificationAppIcon(appId).catch(() => null)
    icons.set(appId, icon)
  }
  return icon
}

/** The sending app's icon, or its first letter while loading or when Windows has none. */
export function AppIcon({ appId, name }: { appId: string; name: string }) {
  const [src, setSrc] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    void loadIcon(appId).then((icon) => !cancelled && setSrc(icon))
    return () => {
      cancelled = true
    }
  }, [appId])

  if (src) {
    return <img src={src} alt="" draggable={false} className="size-4 shrink-0 rounded-[4px]" />
  }
  return (
    <span
      aria-hidden
      className="grid size-4 shrink-0 place-items-center rounded-[4px] bg-panel-separator text-[10px] leading-none font-medium uppercase"
    >
      {name.trim().charAt(0)}
    </span>
  )
}
