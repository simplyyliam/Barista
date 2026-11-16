import type { MouseEvent } from "react"
import { commands } from "@/shared/bindings"

/**
 * Click handlers for any item (client item or tray icon). Left, right and middle go to Rust, which
 * forwards them to the app the way the Windows tray does. There is no double-click handler on
 * purpose: the bar finds a double click itself, from two left clicks on the same icon in a row.
 *
 * @example
 * <div {...itemClickHandlers(item.id)} />
 */
export function itemClickHandlers(id: string) {
  return {
    onClick: () => void commands.itemClicked(id, "left"),
    onContextMenu: (e: MouseEvent) => {
      e.preventDefault()
      void commands.itemClicked(id, "right")
    },
    onAuxClick: (e: MouseEvent) => {
      if (e.button === 1) void commands.itemClicked(id, "middle")
    },
  }
}
