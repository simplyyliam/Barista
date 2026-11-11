import { describe, expect, it } from "vitest"
import type { MenuEntry } from "@/shared/bindings"
import { formatAccelerator } from "./accelerator"
import { firstIndex, inTriangle, lastIndex, nextIndex, typeaheadIndex } from "./navigation"

const row = (label: string, enabled = true): MenuEntry =>
  ({ id: label, kind: "normal", label, checked: false, enabled, submenu: [] }) as unknown as MenuEntry
const separator = { id: null, kind: "separator", label: "", checked: false, enabled: true, submenu: [] } as unknown as MenuEntry

describe("nextIndex", () => {
  const entries = [row("Open"), separator, row("Copy", false), row("Quit")]

  it("skips separators and disabled rows, and wraps", () => {
    expect(nextIndex(entries, 0, 1)).toBe(3)
    expect(nextIndex(entries, 3, 1)).toBe(0)
    expect(nextIndex(entries, 0, -1)).toBe(3)
  })

  it("starts at the ends when nothing is highlighted", () => {
    expect(nextIndex(entries, -1, 1)).toBe(0)
    expect(nextIndex(entries, -1, -1)).toBe(3)
  })

  it("returns -1 when nothing can be selected", () => {
    expect(nextIndex([separator, row("A", false)], -1, 1)).toBe(-1)
    expect(nextIndex([], -1, 1)).toBe(-1)
  })

  it("finds the first and last selectable rows", () => {
    expect(firstIndex(entries)).toBe(0)
    expect(lastIndex(entries)).toBe(3)
  })
})

describe("typeaheadIndex", () => {
  const entries = [row("Open"), row("Open Recent"), row("Quit"), row("Copy link"), row("Cut", false)]

  it("jumps to the next row starting with the letter", () => {
    expect(typeaheadIndex(entries, "q", -1)).toBe(2)
    expect(typeaheadIndex(entries, "o", -1)).toBe(0)
  })

  it("cycles when the same letter is typed again", () => {
    expect(typeaheadIndex(entries, "o", 0)).toBe(1)
    expect(typeaheadIndex(entries, "oo", 1)).toBe(0)
  })

  it("narrows with a longer word and ignores disabled rows", () => {
    expect(typeaheadIndex(entries, "cu", 3)).toBe(-1)
    expect(typeaheadIndex(entries, "co", 0)).toBe(3)
    expect(typeaheadIndex(entries, "x", 0)).toBe(-1)
  })
})

describe("inTriangle", () => {
  it("is true heading toward the submenu edge and false away from it", () => {
    const apex = { x: 100, y: 50 }
    const top = { x: 200, y: 0 }
    const bottom = { x: 200, y: 200 }
    expect(inTriangle({ x: 150, y: 60 }, apex, top, bottom)).toBe(true)
    expect(inTriangle({ x: 120, y: 190 }, apex, top, bottom)).toBe(false)
    expect(inTriangle({ x: 50, y: 50 }, apex, top, bottom)).toBe(false)
  })
})

describe("formatAccelerator", () => {
  it("writes Windows names", () => {
    expect(formatAccelerator("CmdOrCtrl+Shift+s")).toBe("Ctrl+Shift+S")
    expect(formatAccelerator("Alt+Return")).toBe("Alt+Enter")
    expect(formatAccelerator("F5")).toBe("F5")
  })
})
