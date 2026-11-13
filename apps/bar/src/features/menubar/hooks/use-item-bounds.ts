import { useLayoutEffect } from "react"
import { commands, type ItemBounds } from "@/shared/bindings"

/**
 * Where an element sits in the page, as if no transform were applied. Pills dip while pressed and
 * glide when a neighbour appears; Rust must get the place the pill ends up in, not a frame of the
 * animation. (A transform is a translate plus a scale around the centre, which is what motion uses.)
 */
function layoutRect(element: HTMLElement) {
  const rect = element.getBoundingClientRect()
  const css = getComputedStyle(element).transform
  if (css === "none") return rect
  const m = new DOMMatrixReadOnly(css)
  const width = rect.width / (m.a || 1)
  const height = rect.height / (m.d || 1)
  return {
    left: rect.left - m.e - (width - rect.width) / 2,
    top: rect.top - m.f - (height - rect.height) / 2,
    width,
    height,
  }
}

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
        const rect = layoutRect(element)
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
