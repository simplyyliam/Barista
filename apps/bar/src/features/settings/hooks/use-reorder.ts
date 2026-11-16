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
 * you drag; the new order is saved when you let go. Without a mouse: focus an item, then Alt+Left /
 * Alt+Right move it and Delete removes it, and a live region says what happened (WCAG 2.5.7: every
 * drag has a way that needs no dragging).
 */
export function useReorder({ ids, onReorder, onRemove, boundsRef, name }: Options) {
  const [order, setOrder] = useState<string[] | null>(null)
  const [dragging, setDragging] = useState<string | null>(null)
  const [announcement, setAnnouncement] = useState("")
  const latest = useRef<string[] | null>(null)
  const press = useRef<{ id: string; x: number; y: number; dragging: boolean } | null>(null)
  const justDragged = useRef(false)

  // Rust answered with the saved order (or the items changed): draw what it says again.
  const key = ids.join("|")
  useEffect(() => {
    latest.current = null
    setOrder(null)
  }, [key])

  const local = order && order.length === ids.length && order.every((id) => ids.includes(id)) ? order : null
  const shown = local ?? ids

  const move = (id: string, by: -1 | 1) => {
    const from = shown.indexOf(id)
    const to = from + by
    if (from < 0 || to < 0 || to >= shown.length) return
    const next = [...shown]
    next.splice(from, 1)
    next.splice(to, 0, id)
    onReorder(next)
    setAnnouncement(`${name(id)} moved to position ${to + 1} of ${next.length}`)
  }

  const handlers = (id: string) => ({
    "data-reorder-id": id,
    onPointerDown: (e: PointerEvent<HTMLElement>) => {
      if (e.button !== 0) return
      press.current = { id, x: e.clientX, y: e.clientY, dragging: false }
      e.currentTarget.setPointerCapture(e.pointerId)
    },
    onPointerMove: (e: PointerEvent<HTMLElement>) => {
      const current = press.current
      if (!current || current.id !== id) return
      if (!current.dragging) {
        if (Math.hypot(e.clientX - current.x, e.clientY - current.y) < DRAG_START_PX) return
        current.dragging = true
        setDragging(id)
      }
      const over = document
        .elementFromPoint(e.clientX, e.clientY)
        ?.closest<HTMLElement>("[data-reorder-id]")?.dataset.reorderId
      if (!over || over === id) return
      const base = latest.current ?? shown
      const from = base.indexOf(id)
      const to = base.indexOf(over)
      if (from < 0 || to < 0) return
      const next = [...base]
      next.splice(from, 1)
      next.splice(to, 0, id)
      latest.current = next
      setOrder(next)
    },
    onPointerUp: (e: PointerEvent<HTMLElement>) => {
      const current = press.current
      press.current = null
      setDragging(null)
      if (!current?.dragging) return
      // The click that follows the release belongs to the drag: swallow it.
      justDragged.current = true
      setTimeout(() => (justDragged.current = false), 0)
      const strip = boundsRef?.current?.getBoundingClientRect()
      const off =
        strip &&
        onRemove &&
        (e.clientY < strip.top - OFF_BAR_PX || e.clientY > strip.bottom + OFF_BAR_PX)
      if (off) {
        latest.current = null
        setOrder(null)
        onRemove(id)
        setAnnouncement(`${name(id)} removed`)
        return
      }
      const next = latest.current
      if (next) {
        onReorder(next)
        setAnnouncement(`${name(id)} moved to position ${next.indexOf(id) + 1} of ${next.length}`)
      }
    },
    onPointerCancel: () => {
      press.current = null
      setDragging(null)
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
      } else if ((e.key === "Delete" || e.key === "Backspace") && onRemove) {
        e.preventDefault()
        onRemove(id)
        setAnnouncement(`${name(id)} removed`)
      }
    },
  })

  return { shown, dragging, handlers, announcement, move }
}
