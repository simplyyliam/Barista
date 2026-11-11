// Pure keyboard helpers for menus: which row is next, which row a typed letter jumps to,
// whether the pointer is heading for a submenu. No React, so they are easy to test.

import type { MenuEntry } from "@/shared/bindings"

export type Point = { x: number; y: number }

/** Rows the keyboard can land on: not separators, not disabled. */
export function isSelectable(entry: MenuEntry): boolean {
  return entry.kind !== "separator" && entry.enabled
}

export function firstIndex(entries: MenuEntry[]): number {
  return entries.findIndex(isSelectable)
}

export function lastIndex(entries: MenuEntry[]): number {
  return entries.findLastIndex(isSelectable)
}

/**
 * The next selectable row, wrapping around. `from` can be -1 (nothing highlighted yet):
 * down lands on the first row, up on the last. Returns -1 if nothing is selectable.
 */
export function nextIndex(entries: MenuEntry[], from: number, direction: 1 | -1): number {
  const count = entries.length
  if (count === 0) return -1
  const start = from < 0 ? (direction === 1 ? -1 : count) : from
  for (let step = 1; step <= count; step += 1) {
    const index = (((start + direction * step) % count) + count) % count
    const entry = entries[index]
    if (entry && isSelectable(entry)) return index
  }
  return -1
}

/**
 * Type-to-jump: the row a typed `buffer` ("g", "gi"...) points at, or -1.
 * Typing the same letter again ("g", "g") cycles through the rows that start with it.
 */
export function typeaheadIndex(entries: MenuEntry[], buffer: string, from: number): number {
  const text = buffer.toLowerCase()
  if (!text) return -1
  const cycling = [...text].every((c) => c === text[0])
  const needle = cycling ? text[0]! : text
  const count = entries.length
  // Cycling starts after the current row; a growing word ("gi" -> "git") may stay on it.
  const start = cycling ? from + 1 : Math.max(from, 0)
  for (let step = 0; step < count; step += 1) {
    const index = (((start + step) % count) + count) % count
    const entry = entries[index]
    if (entry && isSelectable(entry) && entry.label.trim().toLowerCase().startsWith(needle)) {
      return index
    }
  }
  return -1
}

/** Where the submenu window is, in the main menu window's logical px. */
export type AimTarget = { side: "right" | "left"; x: number; y: number; width: number; height: number }

/** Extra angle around the corner-to-corner cone, so a slightly wobbly hand still counts. */
const AIM_TOLERANCE = (15 * Math.PI) / 180

/**
 * Is the pointer step `prev -> next` heading for the submenu? (React Aria's method.)
 * Angles are measured from `prev`, with x mirrored so the submenu is always on the right:
 * the step is "aimed" when its angle lies between the angles to the submenu's near top and
 * bottom corners, widened by 15 degrees. Moving away from the near edge is never aimed.
 */
export function isAimedAtSubmenu(prev: Point, next: Point, target: AimTarget): boolean {
  const edgeX = target.side === "right" ? target.x : target.x + target.width
  const mirror = target.side === "right" ? 1 : -1
  const toEdge = (edgeX - prev.x) * mirror
  const dx = (next.x - prev.x) * mirror
  const dy = next.y - prev.y
  if (toEdge <= 0) return true // already level with or past the edge
  if (dx < 0) return false // moving away from the submenu
  if (dx === 0 && dy === 0) return true // no movement says nothing
  const heading = Math.atan2(dy, dx)
  const top = Math.atan2(target.y - prev.y, toEdge)
  const bottom = Math.atan2(target.y + target.height - prev.y, toEdge)
  return heading >= top - AIM_TOLERANCE && heading <= bottom + AIM_TOLERANCE
}
