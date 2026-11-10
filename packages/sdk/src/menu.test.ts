import { describe, expect, it } from "vitest"
import { Menu, toAcceleratorText, type MenuItemOptions } from "./menu.js"

const SVG = Buffer.from("<svg xmlns='http://www.w3.org/2000/svg'/>")

describe("Menu.fromTemplate", () => {
  it("builds every entry type and sends only ids and fields over the wire", () => {
    const click = () => {}
    const menu = Menu.fromTemplate([
      { id: "start", label: "Start", accelerator: "CmdOrCtrl+S", click },
      { id: "sounds", label: "Sounds", type: "checkbox", checked: true },
      { type: "separator" },
      { id: "more", label: "More", submenu: [{ id: "r1", type: "radio", label: "A", icon: SVG }] },
      { role: "quit" },
    ])
    const wire = menu.toWire()
    expect(wire[0]).toEqual({ id: "start", label: "Start", accelerator: "Ctrl+S" })
    expect(wire[1]).toEqual({ id: "sounds", type: "checkbox", label: "Sounds", checked: true })
    expect(wire[2]).toEqual({ type: "separator" })
    expect(wire[3]?.type).toBe("submenu")
    expect(wire[3]?.submenu?.[0]?.icon).toMatch(/^data:image\/svg\+xml;base64,/)
    expect(wire[4]).toMatchObject({ role: "quit" })
    expect(JSON.stringify(wire)).not.toContain("click")
    expect(menu.getItemById("start")?.click).toBe(click)
    expect(menu.getItemById("r1")?.label).toBe("A")
  })

  it("makes up ids that never collide with yours", () => {
    const menu = Menu.fromTemplate([
      { label: "x" },
      { id: "menu-1", label: "mine" },
      { label: "y" },
    ])
    expect(menu.items.map((i) => i.id)).toEqual(["menu-2", "menu-1", "menu-3"])
  })

  it("names roles after themselves and keeps separators out of the numbering", () => {
    const menu = Menu.fromTemplate([
      { type: "separator" },
      { label: "x" },
      { role: "about" },
      { id: "quit", label: "mine" },
      { role: "quit" },
    ])
    expect(menu.items.map((i) => i.id)).toEqual([
      "separator:1",
      "menu-1",
      "about",
      "quit",
      "menu-2",
    ])
  })

  it("enforces the bar's rules", () => {
    const deep: MenuItemOptions[] = [
      {
        label: "1",
        submenu: [{ label: "2", submenu: [{ label: "3", submenu: [{ label: "4" }] }] }],
      },
    ]
    expect(() => Menu.fromTemplate(deep)).toThrow(/3 levels/)
    const many = Array.from({ length: 51 }, (_, i) => ({ label: String(i) }))
    expect(() => Menu.fromTemplate(many)).toThrow(/50 entries/)
    expect(() => Menu.fromTemplate([{ id: "a" }, { id: "a" }])).toThrow(/twice/)
    expect(() => Menu.fromTemplate([{ id: "Bad Id" }])).toThrow(TypeError)
    expect(() => Menu.fromTemplate([{ label: "x", checked: true }])).toThrow(/checkbox and radio/)
    expect(() => Menu.fromTemplate([{ type: "submenu" }])).toThrow(/submenu list/)
    expect(() => Menu.fromTemplate([{ accelerator: "x".repeat(40) }])).toThrow(/accelerator/)
    const bigIcon = Buffer.concat([SVG, Buffer.alloc(20 * 1024, 32)])
    expect(() => Menu.fromTemplate([{ icon: bigIcon }])).toThrow(/16 KB/)
  })
})

describe("accelerators", () => {
  it("shows Windows names", () => {
    expect(toAcceleratorText("CommandOrControl+Shift+P")).toBe("Ctrl+Shift+P")
    expect(toAcceleratorText("Option+F4")).toBe("Alt+F4")
    expect(toAcceleratorText("Super+Space")).toBe("Win+Space")
  })
})

describe("applyClick", () => {
  it("mirrors checkbox and radio state from the bar", () => {
    const menu = Menu.fromTemplate([
      { id: "a", type: "radio", checked: true },
      { id: "b", type: "radio" },
      { type: "separator" },
      { id: "c", type: "radio", checked: true },
      { id: "sub", submenu: [{ id: "box", type: "checkbox" }] },
    ])
    menu.applyClick("b", true)
    expect(["a", "b", "c"].map((id) => menu.getItemById(id)?.checked)).toEqual([false, true, true])
    menu.applyClick("box", true)
    expect(menu.getItemById("box")?.checked).toBe(true)
    expect(menu.applyClick("nope", true)).toBeUndefined()
  })
})

describe("toCheckedWire", () => {
  it("re-checks entries changed after fromTemplate", () => {
    const menu = Menu.fromTemplate([{ id: "a", label: "ok" }])
    expect(menu.toCheckedWire()).toEqual([{ id: "a", label: "ok" }])
    menu.getItemById("a")!.label = "x".repeat(200)
    expect(() => menu.toCheckedWire()).toThrow(/label/)
    menu.getItemById("a")!.label = "ok"
    menu.getItemById("a")!.checked = true
    expect(() => menu.toCheckedWire()).toThrow(/checkbox and radio/)
  })
})
