import { describe, expect, it } from "vitest"
import { DEFAULT_CONTROL_CENTER, gridRows } from "./layout"

describe("gridRows", () => {
  it("is two rows for the designed layout", () => {
    expect(gridRows(DEFAULT_CONTROL_CENTER.tiles.map((t) => (t.size === "wide" ? 2 : 1)))).toBe(2)
  })

  it("is zero rows without tiles and one row for a short run", () => {
    expect(gridRows([])).toBe(0)
    expect(gridRows([1, 1, 2])).toBe(1)
  })

  it("moves a wide tile to the next row when it does not fit", () => {
    // 1 1 1 then a wide tile: only one column is left in the first row.
    expect(gridRows([1, 1, 1, 2])).toBe(2)
    expect(gridRows([2, 2, 2])).toBe(2)
  })
})
