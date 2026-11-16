import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent, type PointerEvent, type RefObject } from "react"

/** How far the pointer must travel before a press becomes a drag (a click must stay a click). */
const DRAG_START_PX = 5
/** Released this far outside the editor's strip = the item is dragged off (removed), like macOS. */
const OFF_BAR_PX = 28

interface Options {
  /** The ids in the saved order. */
  ids: string[]
  /** Called once when a drag or a keyboard move ends with a new order. */
  onReorder: (next: string[]) => void
  /** Called when an item is dragged off the strip, or Delete is pressed on it. */
  onRemove?: (id: string) => void
  /** The strip the items sit in; releasing far outside it removes the item. */
  boundsRef?: RefObject<HTMLElement | null>
  /** Words for the screen reader, for example `name(id) => "Wi-Fi"`. */
  name: (id: string) => string
}

/**
 * Drag to reorder with plain pointer events (Tauri turns native drag and drop into file drops, so
 * HTML5 drag does not work here). Items need `data-reorder-id`. The others make room at once while
 * you drag; the new order is saved when you let go.
 *
 * While a drag runs the listeners sit on `window`, not on the item: when the order changes React
 * moves the item's element, and a moved element loses its pointer capture.
 *
 * Without a mouse: focus an item, then Alt+Left / Alt+Right move it and Delete removes it, and a
 * live region says what happened (WCAG 2.5.7: every drag has a way that needs no dragging).
 */
export function useReorder(options: Options) {
  const { ids, boundsRef } = options
  const [order, setOrder] = useState<string[] | null>(null)
  const [dragging, setDragging] = useState<string | null>(null)
  const [announcement, setAnnouncement] = useState("")
  const latest = useRef<string[] | null>(null)
  const justDragged = useRef(false)
  const stop = useRef<(() => void) | null>(null)

  // Rust answered with the saved order (or the items changed): draw what it says again.
  const key = ids.join("|")
  useEffect(() => {
    latest.current = null
    setOrder(null)
  }, [key])

  const local = order && order.length === ids.length && order.every((id) => ids.includes(id)) ? order : null
  const shown = local ?? ids

  // The newest options and order for the window listeners, which outlive a render.
  const live = useRef({ options, shown })
  live.current = { options, shown }

  // A window that closes mid-drag must not leave listeners behind.
  useEffect(() => () => stop.current?.(), [])

  const move = (id: string, by: -1 | 1) => {
    const from = shown.indexOf(id)
    const to = from + by
    if (from < 0 || to < 0 || to >= shown.length) return
    const next = [...shown]
    next.splice(from, 1)
    next.splice(to, 0, id)
    options.onReorder(next)
    setAnnouncement(`${options.name(id)} moved to position ${to + 1} of ${next.length}`)
  }

  const start = (id: string, x0: number, y0: number) => {
    stop.current?.()
    let dragged = false

    const onMove = (e: globalThis.PointerEvent) => {
      if (!dragged) {
        if (Math.hypot(e.clientX - x0, e.clientY - y0) < DRAG_START_PX) return
        dragged = true
        setDragging(id)
      }
      const over = document
        .elementFromPoint(e.clientX, e.clientY)
        ?.closest<HTMLElement>("[data-reorder-id]")?.dataset.reorderId
      if (!over || over === id) return
      const base = latest.current ?? live.current.shown
      const from = base.indexOf(id)
      const to = base.indexOf(over)
      if (from < 0 || to < 0) return
      const next = [...base]
      next.splice(from, 1)
      next.splice(to, 0, id)
      latest.current = next
      setOrder(next)
    }

    const finish = (e: globalThis.PointerEvent | null) => {
      window.removeEventListener("pointermove", onMove)
      window.removeEventListener("pointerup", onUp)
      window.removeEventListener("pointercancel", onCancel)
      stop.current = null
      setDragging(null)
      if (!dragged) return
      // The click that follows the release belongs to the drag: swallow it.
      justDragged.current = true
      setTimeout(() => (justDragged.current = false), 0)
      const { options: o } = live.current
      const strip = boundsRef?.current?.getBoundingClientRect()
      const off =
        e &&
        strip &&
        o.onRemove &&
        (e.clientY < strip.top - OFF_BAR_PX || e.clientY > strip.bottom + OFF_BAR_PX)
      if (off) {
        latest.current = null
        setOrder(null)
        o.onRemove?.(id)
        setAnnouncement(`${o.name(id)} removed`)
        return
      }
      const next = latest.current
      if (next && e) {
        o.onReorder(next)
        setAnnouncement(`${o.name(id)} moved to position ${next.indexOf(id) + 1} of ${next.length}`)
      } else if (!e) {
        latest.current = null
        setOrder(null)
      }
    }
    const onUp = (e: globalThis.PointerEvent) => finish(e)
    const onCancel = () => finish(null)

    window.addEventListener("pointermove", onMove)
    window.addEventListener("pointerup", onUp)
    window.addEventListener("pointercancel", onCancel)
    stop.current = () => finish(null)
  }

  const handlers = (id: string) => ({
    "data-reorder-id": id,
    onPointerDown: (e: PointerEvent<HTMLElement>) => {
      if (e.button !== 0) return
      start(id, e.clientX, e.clientY)
    },
    onClickCapture: (e: MouseEvent) => {
      if (justDragged.current) e.stopPropagation()
    },
    onKeyDown: (e: KeyboardEvent) => {
      if (e.altKey && e.key === "ArrowLeft") {
        e.preventDefault()
        move(id, -1)
      } else if (e.altKey && e.key === "ArrowRight") {
        e.preventDefault()
        move(id, 1)
      } else if ((e.key === "Delete" || e.key === "Backspace") && options.onRemove) {
        e.preventDefault()
        options.onRemove(id)
        setAnnouncement(`${options.name(id)} removed`)
      }
    },
  })

  return { shown, dragging, handlers, announcement, move }
}
