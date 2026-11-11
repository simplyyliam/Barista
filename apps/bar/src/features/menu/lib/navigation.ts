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

function sign(a: Point, b: Point, c: Point): number {
  return (a.x - c.x) * (b.y - c.y) - (b.x - c.x) * (a.y - c.y)
}

/** Is `p` inside the triangle a-b-c (edges included)? */
export function inTriangle(p: Point, a: Point, b: Point, c: Point): boolean {
  const d1 = sign(p, a, b)
  const d2 = sign(p, b, c)
  const d3 = sign(p, c, a)
  const hasNegative = d1 < 0 || d2 < 0 || d3 < 0
  const hasPositive = d1 > 0 || d2 > 0 || d3 > 0
  return !(hasNegative && hasPositive)
}
