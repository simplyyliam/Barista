import { useEffect, useState, type CSSProperties, type ReactNode, type RefObject } from "react"
import {
  closestCenter,
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core"
import {
  arrayMove,
  horizontalListSortingStrategy,
  rectSortingStrategy,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
} from "@dnd-kit/sortable"
import { CSS } from "@dnd-kit/utilities"

/** Released this far above or below the strip = the item is dragged off (removed), like macOS. */
const OFF_STRIP_PX = 28

/** What an item needs to be draggable: put `setNodeRef` and `style` on its root, `handle` on the part you grab. */
export interface SortableState {
  setNodeRef: (element: HTMLElement | null) => void
  style: CSSProperties
  /** Pointer and keyboard listeners plus the ARIA attributes (Space lifts, arrows move, Space drops, Esc cancels). */
  handle: Record<string, unknown>
  dragging: boolean
}

function Item({ id, render }: { id: string; render: (state: SortableState) => ReactNode }) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id })
  return render({
    setNodeRef,
    style: { transform: CSS.Translate.toString(transform), transition },
    handle: { ...attributes, ...listeners },
    dragging: isDragging,
  })
}

const sameSet = (a: string[], b: string[]) => a.length === b.length && a.every((id) => b.includes(id))

/**
 * A row or grid of things you can drag to reorder, built on dnd-kit: the others glide out of the way,
 * a copy follows the pointer and settles into its place. Dragging an item off the strip removes it
 * (when `onRemove` is given). The keyboard works too (Space to pick up, arrows, Space to drop), with
 * announcements for screen readers.
 */
export function SortableList({
  ids,
  layout,
  onReorder,
  onRemove,
  boundsRef,
  name,
  className,
  renderItem,
  renderOverlay,
}: {
  ids: string[]
  /** `row` for a horizontal strip, `grid` for tiles. */
  layout: "row" | "grid"
  onReorder: (next: string[]) => void
  onRemove?: (id: string) => void
  boundsRef?: RefObject<HTMLElement | null>
  name: (id: string) => string
  className?: string
  renderItem: (id: string, state: SortableState) => ReactNode
  renderOverlay?: (id: string) => ReactNode
}) {
  // The order you just dropped, until Rust answers with the saved one (so nothing snaps back).
  const [local, setLocal] = useState<string[] | null>(null)
  const key = ids.join("|")
  useEffect(() => setLocal(null), [key])
  const shown = local && sameSet(local, ids) ? local : ids

  const [active, setActive] = useState<string | null>(null)
  const sensors = useSensors(
    // A few pixels of travel make a drag, so a click still selects.
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
      // Enter stays a plain "select".
      keyboardCodes: { start: ["Space"], cancel: ["Escape"], end: ["Space"] },
    }),
  )

  const finish = ({ active: dragged, over }: DragEndEvent) => {
    setActive(null)
    const id = String(dragged.id)
    const rect = dragged.rect.current.translated
    const strip = boundsRef?.current?.getBoundingClientRect()
    if (rect && strip && onRemove) {
      const centre = rect.top + rect.height / 2
      if (centre < strip.top - OFF_STRIP_PX || centre > strip.bottom + OFF_STRIP_PX) {
        onRemove(id)
        return
      }
    }
    if (over && dragged.id !== over.id) {
      const next = arrayMove(shown, shown.indexOf(id), shown.indexOf(String(over.id)))
      setLocal(next)
      onReorder(next)
    }
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragStart={(e) => setActive(String(e.active.id))}
      onDragEnd={finish}
      onDragCancel={() => setActive(null)}
      accessibility={{
        announcements: {
          onDragStart: ({ active: a }) => `Picked up ${name(String(a.id))}.`,
          onDragOver: ({ active: a, over }) =>
            over ? `${name(String(a.id))} is over position ${shown.indexOf(String(over.id)) + 1} of ${shown.length}.` : undefined,
          onDragEnd: ({ active: a, over }) =>
            over ? `${name(String(a.id))} dropped at position ${shown.indexOf(String(over.id)) + 1}.` : `${name(String(a.id))} dropped.`,
          onDragCancel: ({ active: a }) => `Moving ${name(String(a.id))} cancelled.`,
        },
      }}
    >
      <SortableContext items={shown} strategy={layout === "row" ? horizontalListSortingStrategy : rectSortingStrategy}>
        <div className={className}>
          {shown.map((id) => (
            <Item key={id} id={id} render={(state) => renderItem(id, state)} />
          ))}
        </div>
      </SortableContext>
      <DragOverlay dropAnimation={{ duration: 200, easing: "cubic-bezier(0.2, 0, 0, 1)" }}>
        {active && renderOverlay ? renderOverlay(active) : null}
      </DragOverlay>
    </DndContext>
  )
}
