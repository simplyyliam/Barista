// The lifecycle: the bar goes away and comes back, the client starts first, the bar says
// goodbye. A fake bar on a real named pipe plays the bar; "killing" it is closing the server.
import { randomUUID } from "node:crypto"
import { afterEach, describe, expect, it } from "vitest"
import { FakeBar } from "../test/fake-bar.js"
import type { ClientMessage } from "./protocol.js"
import { StatusBar, retryDelay } from "./status-bar.js"
import { StatusItem } from "./status-item.js"

const bars: StatusBar[] = []
const fakes: FakeBar[] = []

afterEach(async () => {
  bars.splice(0).forEach((bar) => bar.close())
  await Promise.all(fakes.splice(0).map((fake) => fake.stop()))
})

async function startBar(pipeName?: string): Promise<FakeBar> {
  const fake = new FakeBar(pipeName)
  fakes.push(fake)
  await fake.listen()
  return fake
}

async function connect(fake: FakeBar | string): Promise<StatusBar> {
  const pipeName = typeof fake === "string" ? fake : fake.pipeName
  const bar = await StatusBar.connect({ appName: "Test app", pipeName, timeoutMs: 500 })
  bars.push(bar)
  return bar
}

const until = async (check: () => boolean, ms = 3000) => {
  const end = Date.now() + ms
  while (!check() && Date.now() < end) await new Promise((r) => setTimeout(r, 10))
  expect(check()).toBe(true)
}

const types = (fake: FakeBar) => fake.received.map((m) => m.type)
const unusedPipe = () => `\\\\.\\pipe\\barista-test-${randomUUID()}`

describe("the bar is killed and started again", () => {
  it("puts every item back with its menu, popover and last state", async () => {
    const first = await startBar()
    const bar = await connect(first)
    const a = await StatusItem.create({ id: "a", label: "0", tooltip: "old", bar })
    const b = await StatusItem.create({ id: "b", label: "hidden", bar })
    a.setLabel("42")
    a.setTooltip("") // cleared: must stay cleared after the restart
    b.setVisible(false)
    a.setMenu([
      { id: "start", label: "Start" },
      { id: "sounds", label: "Sounds", type: "checkbox", checked: false },
    ])
    b.setPopover({ window: { show() {}, hide() {}, onClose() {} }, width: 320, height: 380 })

    // The user ticks "Sounds" once: the menu now has it checked.
    first.send({ type: "menu.click", id: "a", menuItemId: "sounds", checked: true })
    await new Promise((r) => setTimeout(r, 30))

    let gone = 0
    let back = 0
    bar.on("unavailable", () => gone++).on("available", () => back++)
    const pipeName = first.pipeName
    await first.stop() // the bar is killed
    await until(() => !bar.available)
    expect(gone).toBe(1)

    const second = await startBar(pipeName) // and started again
    await until(() => bar.available)
    expect(back).toBe(1)
    expect(bar.sessionId).toBe("s1")

    // Everything is said again, in order: hello, a (create, menu), b (create, hidden, popover).
    await until(() => second.received.length >= 6)
    const [hello, createA, setMenuA, createB, hideB, popoverB] = second.received as ClientMessage[]
    expect(hello).toMatchObject({ type: "hello" })
    expect(createA).toMatchObject({ type: "item.create", id: "a", label: "42" })
    expect(createA).not.toHaveProperty("tooltip")
    expect(setMenuA).toMatchObject({ type: "item.setMenu", id: "a" })
    expect(JSON.stringify(setMenuA)).toContain('"checked":true')
    expect(createB).toMatchObject({ type: "item.create", id: "b", label: "hidden" })
    expect(hideB).toMatchObject({ type: "item.update", id: "b", visible: false })
    expect(popoverB).toMatchObject({ type: "item.setPopover", id: "b", width: 320, height: 380 })
  })

  it("keeps the callbacks: a click after the restart reaches the old listener", async () => {
    const first = await startBar()
    const bar = await connect(first)
    const item = await StatusItem.create({ id: "a", bar })
    const clicks: string[] = []
    item.on("click", ({ button }) => clicks.push(button))
    item.setMenu([{ id: "go", label: "Go", click: () => clicks.push("menu") }])
    const pipeName = first.pipeName
    await first.stop()
    await until(() => !bar.available)
    const second = await startBar(pipeName)
    await until(() => bar.available)
    second.send({ type: "item.click", id: "a", button: "middle" })
    second.send({ type: "menu.click", id: "a", menuItemId: "go" })
    await until(() => clicks.length === 2)
    expect(clicks).toEqual(["middle", "menu"])
  })

  it("items changed while the bar was away are sent as their latest value, nothing queued", async () => {
    const first = await startBar()
    const bar = await connect(first)
    const item = await StatusItem.create({ id: "a", label: "1", bar })
    const pipeName = first.pipeName
    await first.stop()
    await until(() => !bar.available)
    // Silent while away: no throw, nothing sent.
    for (let n = 2; n <= 50; n++) item.setLabel(String(n))
    const second = await startBar(pipeName)
    await until(() => second.received.length >= 2)
    await new Promise((r) => setTimeout(r, 50))
    expect(types(second)).toEqual(["hello", "item.create"]) // one create, no 49 updates
    expect(second.received[1]).toMatchObject({ label: "50" })
  })

  it("an item removed while the bar was away is not brought back", async () => {
    const first = await startBar()
    const bar = await connect(first)
    const gone = await StatusItem.create({ id: "gone", bar })
    await StatusItem.create({ id: "stays", bar })
    const pipeName = first.pipeName
    await first.stop()
    await until(() => !bar.available)
    gone.remove()
    const second = await startBar(pipeName)
    await until(() => second.received.length >= 2)
    await new Promise((r) => setTimeout(r, 50))
    expect(second.received.filter((m) => m.type === "item.create").map((m) => (m as { id: string }).id)).toEqual(["stays"])
  })

  it("comes back within 2 seconds of the bar starting", async () => {
    const first = await startBar()
    const bar = await connect(first)
    await StatusItem.create({ id: "a", bar })
    const pipeName = first.pipeName
    await first.stop()
    await until(() => !bar.available)
    // Let it settle into the slowest retry rhythm (a try every half second) first.
    await new Promise((r) => setTimeout(r, 2500))
    const started = Date.now()
    const second = await startBar(pipeName)
    await second.next((m) => m.type === "item.create")
    expect(Date.now() - started).toBeLessThan(2000)
  }, 15_000)

  it("closes an open popover and menu when the bar goes, and runs close", async () => {
    const first = await startBar()
    const bar = await connect(first)
    const item = await StatusItem.create({ id: "a", bar })
    const calls: string[] = []
    item.setPopover({
      window: { show: () => void calls.push("show"), hide: () => void calls.push("hide"), onClose() {} },
      width: 320,
      height: 380,
    })
    const events: string[] = []
    item.on("open", () => events.push("open")).on("close", () => events.push("close"))
    first.send({
      type: "popover.show",
      id: "a",
      rect: { x: 0, y: 0, width: 320, height: 380 },
      scale: 1,
      monitor: "m1",
      tone: "dark",
    })
    await until(() => calls.includes("show"))
    await first.stop()
    await until(() => calls.includes("hide"))
    expect(events).toEqual(["open", "close"])
  })
})

describe("the client starts before the bar", () => {
  it("connect resolves at once, items can be created, they appear when the bar starts", async () => {
    const pipeName = unusedPipe()
    const bar = await connect(pipeName)
    expect(bar.available).toBe(false)
    const item = await StatusItem.create({ id: "early", label: "hi", bar })
    item.setLabel("hello") // no bar yet: silent
    let back = 0
    bar.on("available", () => back++)

    const fake = await startBar(pipeName)
    await until(() => bar.available)
    await fake.next((m) => m.type === "item.create")
    expect(fake.received[1]).toMatchObject({ type: "item.create", id: "early", label: "hello" })
    expect(back).toBe(1)
  })

  it("never logs or throws while it waits", async () => {
    const warnings: unknown[] = []
    const original = console.warn
    console.warn = (...args: unknown[]) => void warnings.push(args)
    try {
      const bar = await connect(unusedPipe())
      await StatusItem.create({ id: "a", bar })
      await new Promise((r) => setTimeout(r, 700)) // several attempts
    } finally {
      console.warn = original
    }
    expect(warnings).toEqual([])
  })
})

describe("the waiting hint", () => {
  async function hintsWhile(run: () => Promise<void>): Promise<string[]> {
    const lines: string[] = []
    const original = process.stderr.write.bind(process.stderr)
    process.stderr.write = ((chunk: string | Uint8Array) => {
      lines.push(String(chunk))
      return true
    }) as typeof process.stderr.write
    try {
      await run()
    } finally {
      process.stderr.write = original
    }
    return lines.filter((line) => line.includes("[barista]"))
  }

  it("prints once on stderr when no bar shows up, and the connection keeps waiting", async () => {
    const hints = await hintsWhile(async () => {
      const bar = await StatusBar.connect({ pipeName: unusedPipe(), timeoutMs: 200, waitingHintMs: 100 })
      bars.push(bar)
      await new Promise((r) => setTimeout(r, 500))
      expect(bar.available).toBe(false)
    })
    expect(hints).toHaveLength(1)
    expect(hints[0]).toContain("Waiting for the Barista bar")
  })

  it("stays quiet when the bar is there, or arrives before the hint", async () => {
    const hints = await hintsWhile(async () => {
      await connect(await startBar())
      const pipeName = unusedPipe()
      const early = await StatusBar.connect({ pipeName, timeoutMs: 200, waitingHintMs: 400 })
      bars.push(early)
      await startBar(pipeName)
      await until(() => early.available)
      await new Promise((r) => setTimeout(r, 600))
    })
    expect(hints).toEqual([])
  })

  it("stays quiet after close()", async () => {
    const hints = await hintsWhile(async () => {
      const bar = await StatusBar.connect({ pipeName: unusedPipe(), timeoutMs: 200, waitingHintMs: 100 })
      bar.close()
      await new Promise((r) => setTimeout(r, 300))
    })
    expect(hints).toEqual([])
  })
})

describe("the bar says goodbye", () => {
  it("answers ping with pong, echoing the seq", async () => {
    const fake = await startBar()
    await connect(fake)
    fake.send({ type: "ping", seq: 7 })
    expect(await fake.next((m) => m.type === "pong")).toEqual({ type: "pong", seq: 7 })
  })

  it("shutdown quit: waits and reconnects", async () => {
    const fake = await startBar()
    const bar = await connect(fake)
    await StatusItem.create({ id: "a", bar })
    await fake.next((m) => m.type === "item.create") // read before we hang up
    fake.send({ type: "shutdown", reason: "quit" })
    fake.closeClient() // what the bar does right after
    await until(() => fake.connections >= 2) // the SDK is back, on the same (still listening) pipe
    await until(() => bar.available)
    expect(bar.closed).toBe(false)
    await until(() => types(fake).filter((t) => t === "item.create").length === 2)
  })

  it("shutdown too_many_errors: stays down, emits close, never reconnects", async () => {
    const fake = await startBar()
    const bar = await connect(fake)
    let closed = 0
    bar.on("close", () => closed++)
    fake.send({ type: "shutdown", reason: "too_many_errors" })
    await until(() => closed === 1)
    expect(bar.closed).toBe(true)
    await new Promise((r) => setTimeout(r, 400))
    expect(fake.connections).toBe(1)
  })

  it("a refused hello after a restart (version_mismatch) ends the connection with error and close", async () => {
    const first = await startBar()
    const bar = await connect(first)
    const errors: string[] = []
    let closed = 0
    bar.on("error", (e) => errors.push(e.code)).on("close", () => closed++)
    const pipeName = first.pipeName
    await first.stop()
    const second = new FakeBar(pipeName)
    fakes.push(second)
    second.onHello = () => ({ type: "error", requestId: null, code: "version_mismatch", message: "too new" })
    await second.listen()
    await until(() => closed === 1)
    expect(errors).toEqual(["version_mismatch"])
  })
})

describe("close and isAvailable", () => {
  it("close() says bye, emits close once and never reconnects", async () => {
    const fake = await startBar()
    const bar = await connect(fake)
    let closed = 0
    bar.on("close", () => closed++)
    bar.close()
    bar.close()
    await fake.next((m) => m.type === "bye")
    await until(() => closed === 1)
    await new Promise((r) => setTimeout(r, 400))
    expect(closed).toBe(1)
    expect(fake.connections).toBe(1)
  })

  it("close() while waiting for a bar that is not there still emits close", async () => {
    const bar = await connect(unusedPipe())
    let closed = 0
    bar.on("close", () => closed++)
    bar.close()
    await until(() => closed === 1)
  })

  it("isAvailable is true with a bar listening and false without", async () => {
    const fake = await startBar()
    expect(await StatusBar.isAvailable({ pipeName: fake.pipeName })).toBe(true)
    expect(await StatusBar.isAvailable({ pipeName: unusedPipe(), timeoutMs: 200 })).toBe(false)
  })

  it("probing does not make the bar see a client session", async () => {
    const fake = await startBar()
    await StatusBar.isAvailable({ pipeName: fake.pipeName })
    await new Promise((r) => setTimeout(r, 30))
    expect(fake.received).toEqual([]) // connected and left without saying hello
  })
})

describe("retryDelay", () => {
  it("backs off 100, 200, 400 ms and then stays at half a second", () => {
    const middle = () => 0.5 // no jitter
    expect([0, 1, 2, 3, 4, 50].map((n) => retryDelay(n, middle))).toEqual([
      100, 200, 400, 500, 500, 500,
    ])
  })

  it("jitters by at most 20 percent and never goes over half a second", () => {
    for (const n of [0, 1, 2, 3, 10]) {
      for (const random of [0, 0.5, 0.999]) {
        const delay = retryDelay(n, () => random)
        const base = [100, 200, 400][n] ?? 500
        expect(delay).toBeGreaterThanOrEqual(Math.round(base * 0.8))
        expect(delay).toBeLessThanOrEqual(500)
      }
    }
  })
})
