import { useLayoutEffect } from "react"
import { commands } from "@/shared/bindings"

/**
 * Tells Rust how much room the bar has for status items and tray icons (`setItemSpace`), so the ones
 * that do not fit are sent to the overflow list. The room is the right-hand column of the bar minus
 * everything in it that is not an item (battery, Wi-Fi, clock, the tray arrow...). `itemIds` are the ids
 * of all items, so their pills are not counted as fixed. The bar has three columns: the right group
 * is marked `data-bar-right` and the empty middle one `data-bar-center`.
 */
export function useItemSpace(itemIds: string[]) {
  const key = itemIds.join("|")
  useLayoutEffect(() => {
    const ids = new Set(key ? key.split("|") : [])
    let last = ""
    const send = () => {
      const right = document.querySelector<HTMLElement>("[data-bar-right]")
      const center = document.querySelector<HTMLElement>("[data-bar-center]")
      const bar = right?.parentElement
      if (!right || !bar) return
      const barStyle = getComputedStyle(bar)
      const edges = parseFloat(barStyle.paddingLeft) + parseFloat(barStyle.paddingRight)
      const column = (bar.clientWidth - edges - (center?.offsetWidth ?? 0)) / 2
      const gap = parseFloat(getComputedStyle(right).columnGap) || 0
      let fixed = 0
      let fixedCount = 0
      for (const child of Array.from(right.children) as HTMLElement[]) {
        if (ids.has(child.dataset.itemId ?? "")) continue
        fixed += child.offsetWidth
        fixedCount += 1
      }
      const room = Math.max(0, column - fixed - gap * fixedCount)
      const next = `${Math.round(room)}|${gap}`
      if (next === last) return
      last = next
      void commands.setItemSpace(room, gap)
    }
    send()
    const observer = new ResizeObserver(send)
    observer.observe(document.body)
    return () => observer.disconnect()
  }, [key])
}
