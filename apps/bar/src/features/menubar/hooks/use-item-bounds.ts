import { useLayoutEffect } from "react"
import { commands, type ItemBounds } from "@/shared/bindings"

/**
 * Tells Rust where each status item is drawn (logical px, relative to the bar window), so menus
 * open right under their icon. Items carry `data-item-id`; call this after they render.
 */
export function useItemBounds(itemsKey: string) {
  useLayoutEffect(() => {
    let last = ""
    const send = () => {
      const bounds: ItemBounds[] = Array.from(
        document.querySelectorAll<HTMLElement>("[data-item-id]"),
      ).map((element) => {
        const rect = element.getBoundingClientRect()
        return {
          id: element.dataset.itemId ?? "",
          x: rect.left,
          y: rect.top,
          width: rect.width,
          height: rect.height,
        }
      })
      const key = JSON.stringify(bounds)
      if (key === last) return
      last = key
      void commands.setItemBounds(bounds)
    }

    send()
    // Items move when siblings change size (a clock ticking over, a label changing).
    const observer = new ResizeObserver(send)
    observer.observe(document.body)
    document.querySelectorAll<HTMLElement>("[data-item-id]").forEach((element) => {
      observer.observe(element)
      // A sibling appearing or going (volume, a status item) moves this item without resizing it,
      // but it does resize the group the item sits in.
      if (element.parentElement) observer.observe(element.parentElement)
    })
    return () => observer.disconnect()
  }, [itemsKey])
}
