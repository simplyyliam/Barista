import { describe, expect, it, vi } from "vitest"
import { electronPopover, toBounds, type ElectronWindowLike } from "./electron.js"
import type { Placement, PopoverWindow } from "./popover.js"
import type { ClientMessage } from "./protocol.js"
import type { StatusBar } from "./status-bar.js"
import { StatusItem } from "./status-item.js"

const placement: Placement = {
  x: 100,
  y: 36,
  width: 480,
  height: 570,
  scale: 1.5,
  monitor: "m1",
  tone: "dark",
}

/** Just enough of a StatusBar for the item: it records what the item sends. */
function stubBar() {
  const sent: ClientMessage[] = []
  const bar = {
    items: new Map<string, StatusItem>(),
    send: (message: ClientMessage) => void sent.push(message),
    requestIdFor: (id: string) => `${id}#1`,
    quit: () => {},
  } as unknown as StatusBar
  return { bar, sent }
}

function fakeWindow() {
  const calls: string[] = []
  let close: (() => void) | undefined
  let finishShow: (() => void) | undefined
  const win: PopoverWindow & {
    calls: string[]
    escape(): void
    slowShow(): void
    failShow(): void
    failing: boolean
  } = {
    calls,
    failing: false,
    show(p) {
      calls.push(`show ${p.x},${p.y} ${p.width}x${p.height}`)
      if (win.failing) throw new Error("window is gone")
      return finishShow === undefined ? undefined : new Promise<void>((r) => (finishShow = r))
    },
    hide: () => void calls.push("hide"),
    onClose: (listener) => void (close = listener),
    escape: () => close?.(),
    slowShow: () => void (finishShow = () => {}),
    failShow: () => void (win.failing = true),
  }
  return Object.assign(win, { finish: () => finishShow?.() })
}

async function setup() {
  const { bar, sent } = stubBar()
  const item = await StatusItem.create({ id: "note", bar })
  const win = fakeWindow()
  sent.length = 0
  return { item, win, sent }
}

const types = (sent: ClientMessage[]) => sent.map((m) => m.type)

describe("setPopover", () => {
  it("sends the size, and 0 x 0 to remove it", async () => {
    const { item, win, sent } = await setup()
    item.setPopover({ window: win, width: 320, height: 380 })
    expect(sent[0]).toMatchObject({ type: "item.setPopover", id: "note", width: 320, height: 380 })
    item.setPopover(null)
    expect(sent[1]).toMatchObject({ type: "item.setPopover", width: 0, height: 0 })
  })

  it("throws a TypeError for a bad size and sends nothing", async () => {
    const { item, win, sent } = await setup()
    for (const [width, height] of [
      [10, 300],
      [300, 5000],
      [300.5, 300],
      [Number.NaN, 300],
    ] as const) {
      expect(() => item.setPopover({ window: win, width, height })).toThrow(TypeError)
    }
    expect(sent).toEqual([])
  })

  it("listens to a window once, even if set twice", async () => {
    const { item, sent } = await setup()
    const onClose = vi.fn()
    const win: PopoverWindow = { show: () => {}, hide: () => {}, onClose }
    item.setPopover({ window: win, width: 100, height: 100 })
    item.setPopover({ window: win, width: 200, height: 200 })
    expect(onClose).toHaveBeenCalledTimes(1)
    expect(sent).toHaveLength(2)
  })
})

describe("popover.show from the bar", () => {
  it("runs open, shows the window at the rect and confirms with popover.shown", async () => {
    const { item, win, sent } = await setup()
    const events: string[] = []
    item.on("open", () => events.push("open"))
    item.setPopover({ window: win, width: 320, height: 380 })
    sent.length = 0
    await item.emitPopoverShow(placement)
    expect(win.calls).toEqual(["show 100,36 480x570"])
    expect(events).toEqual(["open"])
    expect(sent).toEqual([{ type: "popover.shown", id: "note" }])
  })

  it("waits for an async show before confirming", async () => {
    const { item, win, sent } = await setup()
    item.setPopover({ window: win, width: 320, height: 380 })
    sent.length = 0
    win.slowShow()
    const shown = item.emitPopoverShow(placement)
    await Promise.resolve()
    expect(sent).toEqual([])
    win.finish()
    await shown
    expect(types(sent)).toEqual(["popover.shown"])
  })

  it("does not confirm when the window fails to show, and tells the bar it is closed", async () => {
    const { item, win, sent } = await setup()
    const events: string[] = []
    item.on("close", () => events.push("close"))
    item.setPopover({ window: win, width: 320, height: 380 })
    sent.length = 0
    win.failShow()
    // The error is rethrown on a microtask like any listener error.
    const thrown = new Promise((resolve) => process.once("uncaughtException", resolve))
    await item.emitPopoverShow(placement)
    expect(await thrown).toMatchObject({ message: "window is gone" })
    expect(types(sent)).toEqual(["popover.closed"])
    expect(events).toEqual(["close"])
  })

  it("answers closed when it has no popover, so the icon is not stuck open", async () => {
    const { item, sent } = await setup()
    await item.emitPopoverShow(placement)
    expect(sent).toEqual([{ type: "popover.closed", id: "note" }])
  })
})

describe("closing", () => {
  it("popover.hide hides the window, runs close and sends nothing back", async () => {
    const { item, win, sent } = await setup()
    const events: string[] = []
    item.on("close", () => events.push("close"))
    item.setPopover({ window: win, width: 320, height: 380 })
    await item.emitPopoverShow(placement)
    sent.length = 0
    item.emitPopoverHide()
    expect(win.calls.at(-1)).toBe("hide")
    expect(events).toEqual(["close"])
    expect(sent).toEqual([])
    // A second hide, or one when nothing is open, does nothing.
    item.emitPopoverHide()
    expect(events).toEqual(["close"])
  })

  it("a hide that arrives while the window is still showing wins", async () => {
    const { item, win, sent } = await setup()
    item.setPopover({ window: win, width: 320, height: 380 })
    sent.length = 0
    win.slowShow()
    const shown = item.emitPopoverShow(placement)
    item.emitPopoverHide()
    win.finish()
    await shown
    expect(types(sent)).not.toContain("popover.shown")
    expect(win.calls.filter((c) => c === "hide").length).toBeGreaterThanOrEqual(1)
  })

  it("the window closing itself (Esc) sends popover.closed and runs close", async () => {
    const { item, win, sent } = await setup()
    const events: string[] = []
    item.on("close", () => events.push("close"))
    item.setPopover({ window: win, width: 320, height: 380 })
    await item.emitPopoverShow(placement)
    sent.length = 0
    win.escape()
    expect(sent).toEqual([{ type: "popover.closed", id: "note" }])
    expect(events).toEqual(["close"])
  })

  it("a window that closes while it is not open says nothing", async () => {
    const { item, win, sent } = await setup()
    item.setPopover({ window: win, width: 320, height: 380 })
    sent.length = 0
    win.escape()
    expect(sent).toEqual([])
  })

  it("removing the item hides an open popover", async () => {
    const { item, win } = await setup()
    item.setPopover({ window: win, width: 320, height: 380 })
    await item.emitPopoverShow(placement)
    item.remove()
    expect(win.calls.at(-1)).toBe("hide")
  })
})

describe("open and close for menus, and click anchors", () => {
  it("menu.opened and menu.closed run open and close", async () => {
    const { item } = await setup()
    const events: string[] = []
    item.on("open", () => events.push("open")).on("close", () => events.push("close"))
    item.emitMenuOpened()
    item.emitMenuClosed()
    expect(events).toEqual(["open", "close"])
  })

  it("on('click') gets the anchor when the bar sends one", async () => {
    const { item } = await setup()
    const seen: unknown[] = []
    item.on("click", (event) => seen.push(event))
    item.emitClick("left", { x: -1920, y: 0, width: 24, height: 36 })
    item.emitClick("middle")
    expect(seen).toEqual([
      { button: "left", anchor: { x: -1920, y: 0, width: 24, height: 36 } },
      { button: "middle" },
    ])
  })

  it("off stops open and close listeners", async () => {
    const { item } = await setup()
    const listener = vi.fn()
    item.on("open", listener).off("open", listener)
    item.emitMenuOpened()
    expect(listener).not.toHaveBeenCalled()
  })
})

describe("electron adapter", () => {
  function electronWindow() {
    const calls: string[] = []
    const handlers: Record<string, (...args: never[]) => void> = {}
    const win: ElectronWindowLike = {
      setBounds: (b) => void calls.push(`bounds ${b.x},${b.y} ${b.width}x${b.height}`),
      show: () => void calls.push("show"),
      focus: () => void calls.push("focus"),
      hide: () => void calls.push("hide"),
      on: (event, listener) => void (handlers[event] = listener),
      webContents: { on: (event, listener) => void (handlers[event] = listener as never) },
    }
    return { win, calls, handlers }
  }

  it("moves, shows and focuses, dividing by the scale without `screen`", () => {
    const { win, calls } = electronWindow()
    electronPopover(win).show({ ...placement, x: 150, y: 54, width: 480, height: 570 })
    expect(calls).toEqual(["bounds 100,36 320x380", "show", "focus"])
  })

  it("uses screen.screenToDipRect when given (mixed-DPI monitors)", () => {
    const screenToDipRect = vi.fn((_w: null, r: { x: number; y: number; width: number; height: number }) => ({
      x: r.x / 2,
      y: r.y / 2,
      width: r.width / 2,
      height: r.height / 2,
    }))
    expect(toBounds({ ...placement, scale: 1.5 }, { screenToDipRect })).toEqual({
      x: 50,
      y: 18,
      width: 240,
      height: 285,
    })
    expect(screenToDipRect).toHaveBeenCalledWith(null, { x: 100, y: 36, width: 480, height: 570 })
  })

  it("handles negative coordinates (a monitor left of the primary one)", () => {
    expect(toBounds({ ...placement, x: -1620, y: 0, scale: 1 })).toMatchObject({ x: -1620, y: 0 })
  })

  it("Esc hides the window and reports the close; other keys do nothing", () => {
    const { win, calls, handlers } = electronWindow()
    const popover = electronPopover(win)
    const closed = vi.fn()
    popover.onClose(closed)
    const key = handlers["before-input-event"] as unknown as (
      e: unknown,
      i: { type: string; key: string },
    ) => void
    key({}, { type: "keyDown", key: "a" })
    key({}, { type: "keyUp", key: "Escape" })
    expect(closed).not.toHaveBeenCalled()
    key({}, { type: "keyDown", key: "Escape" })
    expect(calls).toEqual(["hide"])
    expect(closed).toHaveBeenCalledTimes(1)
  })

  it("a destroyed window reports the close", () => {
    const { win, handlers } = electronWindow()
    const popover = electronPopover(win)
    const closed = vi.fn()
    popover.onClose(closed)
    ;(handlers["closed"] as unknown as () => void)()
    expect(closed).toHaveBeenCalledTimes(1)
  })
})
