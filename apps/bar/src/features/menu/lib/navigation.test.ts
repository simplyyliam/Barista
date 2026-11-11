import { describe, expect, it } from "vitest"
import type { MenuEntry } from "@/shared/bindings"
import { formatAccelerator } from "./accelerator"
import { firstIndex, isAimedAtSubmenu, lastIndex, nextIndex, typeaheadIndex } from "./navigation"

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

describe("isAimedAtSubmenu", () => {
  // Submenu to the right: its near edge is x=200, from y=0 to y=200.
  const right = { side: "right", x: 200, y: 0, width: 150, height: 200 } as const
  const left = { side: "left", x: 0, y: 0, width: 150, height: 200 } as const

  it("is true when heading toward the near edge, even diagonally", () => {
    expect(isAimedAtSubmenu({ x: 100, y: 50 }, { x: 130, y: 60 }, right)).toBe(true)
    expect(isAimedAtSubmenu({ x: 100, y: 50 }, { x: 130, y: 120 }, right)).toBe(true)
  })

  it("is false when moving away, or straight along the menu", () => {
    expect(isAimedAtSubmenu({ x: 100, y: 50 }, { x: 80, y: 55 }, right)).toBe(false)
    expect(isAimedAtSubmenu({ x: 100, y: 50 }, { x: 100, y: 120 }, right)).toBe(false)
    expect(isAimedAtSubmenu({ x: 100, y: 50 }, { x: 105, y: 300 }, right)).toBe(false)
  })

  it("forgives 15 degrees around the cone", () => {
    // Cone from (100,50) tops out at atan2(-50,100) = -26.6 deg: -35 deg is inside the tolerance, -45 is not.
    expect(isAimedAtSubmenu({ x: 100, y: 50 }, { x: 130, y: 50 - 21 }, right)).toBe(true)
    expect(isAimedAtSubmenu({ x: 100, y: 50 }, { x: 130, y: 50 - 30 }, right)).toBe(false)
  })

  it("mirrors for a submenu on the left", () => {
    expect(isAimedAtSubmenu({ x: 300, y: 50 }, { x: 270, y: 60 }, left)).toBe(true)
    expect(isAimedAtSubmenu({ x: 300, y: 50 }, { x: 320, y: 55 }, left)).toBe(false)
  })

  it("counts as aimed once level with the edge", () => {
    expect(isAimedAtSubmenu({ x: 210, y: 50 }, { x: 212, y: 400 }, right)).toBe(true)
  })
})

describe("formatAccelerator", () => {
  it("writes Windows names", () => {
    expect(formatAccelerator("CmdOrCtrl+Shift+s")).toBe("Ctrl+Shift+S")
    expect(formatAccelerator("Alt+Return")).toBe("Alt+Enter")
    expect(formatAccelerator("F5")).toBe("F5")
  })
})
