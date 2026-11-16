import { commands, events } from "@/shared/bindings"
import { useSnapshot } from "./use-snapshot"

/** Whether the bar is collapsed: only pinned items stay in the bar, the rest are in the tray panel. */
export function useCollapsed(): boolean {
  const collapsed = useSnapshot(
    () => commands.getCollapsed(),
    (handler) => events.collapsedChanged.listen((e) => handler(e.payload)),
  )
  // Collapsed is the default until Rust answers.
  return collapsed ?? true
}
