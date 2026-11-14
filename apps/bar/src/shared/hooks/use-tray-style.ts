import { commands, events, type TrayStyle } from "@/shared/bindings"
import { useSnapshot } from "./use-snapshot"

/** How tray icons are shown: `"panel"` (an arrow opens a grid under it) or `"inline"` (unfolds in the bar). */
export function useTrayStyle(): TrayStyle {
  const style = useSnapshot(
    () => commands.getTrayStyle(),
    (handler) => events.trayStyleChanged.listen((e) => handler(e.payload)),
  )
  return style ?? "panel"
}
