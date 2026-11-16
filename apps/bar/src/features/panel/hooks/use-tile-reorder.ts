import { useEffect, useRef, useState, type MouseEvent, type PointerEvent } from "react"
import { commands } from "@/shared/bindings"

/** How far the pointer must travel before a press becomes a drag (a click must stay a click). */
const DRAG_START_PX = 5

/**
 * Drag-to-reorder for the tiles of the tray panel. Press a tile and move: it follows the pointer's
 * place in the grid, the others make room at once (no animation, it is a gesture); let go and the
 * new order is saved with `setOrder`. Tiles are found with `data-item-id`.
 *
 * `ids` are the tile ids in the order Rust sent them. Returns the ids to draw and the handlers for
 * each tile. A drag never counts as a click.
 */
export function useTileReorder(ids: string[]) {
  const [order, setOrder] = useState<string[] | null>(null)
  const latest = useRef<string[] | null>(null)
  const press = useRef<{ id: string; x: number; y: number; dragging: boolean } | null>(null)
  const justDragged = useRef(false)

  // Rust answered with the saved order (or the items changed): draw what it says again.
  const key = ids.join("|")
  useEffect(() => {
    latest.current = null
    setOrder(null)
  }, [key])

  // Only trust the local order while it holds exactly the tiles there are.
  const local = order && order.length === ids.length && order.every((id) => ids.includes(id)) ? order : null
  const shown = local ?? ids

  const handlers = (id: string) => ({
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
      }
      const over = document
        .elementFromPoint(e.clientX, e.clientY)
        ?.closest<HTMLElement>("[data-item-id]")?.dataset.itemId
      if (!over || over === id) return
      const base = latest.current ?? ids
      const from = base.indexOf(id)
      const to = base.indexOf(over)
      if (from < 0 || to < 0) return
      const next = [...base]
      next.splice(from, 1)
      next.splice(to, 0, id)
      latest.current = next
      setOrder(next)
    },
    onPointerUp: () => {
      const current = press.current
      press.current = null
      if (!current?.dragging) return
      // The click that follows the release belongs to the drag: swallow it.
      justDragged.current = true
      setTimeout(() => (justDragged.current = false), 0)
      const next = latest.current
      if (!next) return
      void commands.setOrder(next).then((result) => {
        // Refused (an item left meanwhile): go back to what Rust has.
        if (result.status === "error") {
          latest.current = null
          setOrder(null)
        }
      })
    },
    onPointerCancel: () => {
      press.current = null
    },
    onClickCapture: (e: MouseEvent) => {
      if (justDragged.current) e.stopPropagation()
    },
  })

  return { shown, handlers }
}
