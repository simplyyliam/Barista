import { commands, events, type StatusItemView } from "@/shared/bindings"
import { useSnapshot } from "./use-snapshot"

/** Every item in the bar: client apps' items (`source: "app"`) and Windows tray icons (`"tray"`). */
export function useItems(): StatusItemView[] {
  const items = useSnapshot(
    () => commands.getItems(),
    (handler) => events.itemsChanged.listen((e) => handler(e.payload)),
  )
  return items ?? []
}
