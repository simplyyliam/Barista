import { useEffect, useState } from "react"
import { commands, events, type AppMenus } from "@/shared/bindings"
import { useSnapshot } from "./use-snapshot"

/** The app in front and its menu titles (File, Edit, View ...). `null` until Rust answers. */
export const useAppMenus = () =>
  useSnapshot<AppMenus>(commands.getAppMenus, (h) => events.appMenusChanged.listen((e) => h(e.payload)))

/** The key of the item whose menu is open right now (`appmenu/0`, `script/clock.mjs`, ...), or `null`. */
export function useOpenMenuKey() {
  const [open, setOpen] = useState<string | null>(null)
  useEffect(() => {
    let cancelled = false
    const stops: (() => void)[] = []
    void (async () => {
      const opened = await events.menuOpen.listen(({ payload }) => setOpen(payload.itemId))
      const closed = await events.menuClose.listen(() => setOpen(null))
      if (cancelled) {
        opened()
        closed()
        return
      }
      stops.push(opened, closed)
      const current = await commands.getOpenMenu()
      if (!cancelled && current) setOpen(current.itemId)
    })()
    return () => {
      cancelled = true
      stops.forEach((stop) => stop())
    }
  }, [])
  return open
}
