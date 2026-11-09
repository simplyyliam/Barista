// Talks to a fake bar on a real named pipe, so the whole SDK path (connect, hello, frames) runs.
import net from "node:net"
import { randomUUID } from "node:crypto"
import { afterEach, describe, expect, it } from "vitest"
import { FrameDecoder, encodeFrame } from "./frame.js"
import { PROTOCOL_VERSION, type BarMessage, type ClientMessage } from "./protocol.js"
import { BarError, StatusBar } from "./status-bar.js"
import { StatusItem } from "./status-item.js"

const SVG = Buffer.from("<svg xmlns='http://www.w3.org/2000/svg'/>")

/** A bar that records what clients send and lets the test answer. */
class FakeBar {
  readonly pipeName = `\\\\.\\pipe\\barista-test-${randomUUID()}`
  readonly received: ClientMessage[] = []
  readonly #server: net.Server
  #socket?: net.Socket
  #waiters: Array<() => void> = []
  /** What to answer to `hello`. */
  onHello: (hello: ClientMessage) => BarMessage | null = () => ({
    type: "welcome",
    sessionId: "s1",
    barVersion: "9.9.9",
    protocolVersion: PROTOCOL_VERSION,
  })

  constructor() {
    this.#server = net.createServer((socket) => {
      this.#socket = socket
      const decoder = new FrameDecoder()
      socket.on("data", (chunk) => {
        for (const message of decoder.push(chunk) as ClientMessage[]) {
          this.received.push(message)
          if (message.type === "hello") {
            const reply = this.onHello(message)
            if (reply) socket.write(encodeFrame(reply))
          }
        }
        this.#waiters.splice(0).forEach((wake) => wake())
      })
      socket.on("error", () => {})
    })
  }

  listen(): Promise<void> {
    return new Promise((resolve) => this.#server.listen(this.pipeName, resolve))
  }

  send(message: BarMessage): void {
    this.#socket?.write(encodeFrame(message))
  }

  sendRaw(bytes: Buffer): void {
    this.#socket?.write(bytes)
  }

  closeClient(): void {
    this.#socket?.destroy()
  }

  /** Resolves when `count` messages have arrived. */
  async messages(count: number): Promise<ClientMessage[]> {
    while (this.received.length < count) {
      await new Promise<void>((resolve) => this.#waiters.push(resolve))
    }
    return this.received
  }

  close(): void {
    this.#socket?.destroy()
    this.#server.close()
  }
}

let fake: FakeBar | undefined
const bars: StatusBar[] = []

afterEach(() => {
  bars.splice(0).forEach((bar) => bar.close())
  fake?.close()
  fake = undefined
})

async function setup(): Promise<{ fake: FakeBar; bar: StatusBar }> {
  fake = new FakeBar()
  await fake.listen()
  const bar = await StatusBar.connect({ appName: "Test app", pipeName: fake.pipeName })
  bars.push(bar)
  return { fake, bar }
}

const until = async (check: () => boolean) => {
  for (let i = 0; i < 100 && !check(); i++) await new Promise((r) => setTimeout(r, 10))
  expect(check()).toBe(true)
}

describe("connecting", () => {
  it("sends hello and exposes the welcome", async () => {
    const { fake, bar } = await setup()
    expect(bar.sessionId).toBe("s1")
    expect(bar.barVersion).toBe("9.9.9")
    const [hello] = await fake.messages(1)
    expect(hello).toMatchObject({
      type: "hello",
      protocolVersion: PROTOCOL_VERSION,
      appName: "Test app",
      pid: process.pid,
      sdk: { name: "@barista/sdk" },
    })
  })

  it("rejects with a clear message when the bar is not running", async () => {
    const pipeName = `\\\\.\\pipe\\barista-test-${randomUUID()}`
    await expect(StatusBar.connect({ pipeName })).rejects.toThrow(/Is it running/)
  })

  it("rejects with the bar's error when it refuses hello", async () => {
    fake = new FakeBar()
    fake.onHello = () => ({
      type: "error",
      requestId: null,
      code: "version_mismatch",
      message: "too new",
    })
    await fake.listen()
    const failure = await StatusBar.connect({ pipeName: fake.pipeName }).catch((e: unknown) => e)
    expect(failure).toBeInstanceOf(BarError)
    expect(failure).toMatchObject({ code: "version_mismatch", message: "too new" })
  })

  it("rejects when the bar never answers", async () => {
    fake = new FakeBar()
    fake.onHello = () => null // stays silent
    await fake.listen()
    await expect(StatusBar.connect({ pipeName: fake.pipeName, timeoutMs: 100 })).rejects.toThrow(
      /in time|closed/,
    )
  })
})

describe("StatusItem", () => {
  it("create, update and remove send the right messages", async () => {
    const { fake, bar } = await setup()
    const item = await StatusItem.create({
      id: "counter",
      label: "0",
      icon: SVG,
      tooltip: "tip",
      bar,
    })
    item.setLabel("1")
    item.setIcon("")
    item.setTooltip("new")
    item.setVisible(false)
    item.remove()
    const [, create, label, icon, tooltip, visible, remove] = await fake.messages(7)
    expect(create).toMatchObject({ type: "item.create", id: "counter", label: "0", tooltip: "tip" })
    expect((create as { icon: string }).icon).toMatch(/^data:image\/svg\+xml;base64,/)
    expect(label).toMatchObject({ type: "item.update", id: "counter", label: "1" })
    expect(icon).toMatchObject({ type: "item.update", id: "counter", icon: "" })
    expect(tooltip).toMatchObject({ type: "item.update", tooltip: "new" })
    expect(visible).toMatchObject({ type: "item.update", visible: false })
    expect(remove).toMatchObject({ type: "item.remove", id: "counter" })
  })

  it("leaves out fields that were not given", async () => {
    const { fake, bar } = await setup()
    await StatusItem.create({ id: "a", bar })
    const [, create] = await fake.messages(2)
    expect(Object.keys(create as object).sort()).toEqual(["id", "requestId", "type"])
  })

  it("rejects bad arguments before sending anything", async () => {
    const { fake, bar } = await setup()
    await expect(StatusItem.create({ id: "Bad Id", bar })).rejects.toThrow(TypeError)
    await expect(StatusItem.create({ id: "a", icon: Buffer.from("nope"), bar })).rejects.toThrow(
      /PNG or an SVG/,
    )
    await expect(StatusItem.create({ id: "a", label: "x".repeat(500), bar })).rejects.toThrow(
      /longer/,
    )
    const item = await StatusItem.create({ id: "ok", bar })
    expect(() => item.setLabel("x".repeat(500))).toThrow(/longer/)
    await expect(StatusItem.create({ id: "ok", bar })).rejects.toThrow(/already has an item/)
    await fake.messages(2) // hello + the one good create, nothing else
    await new Promise((r) => setTimeout(r, 50))
    expect(fake.received).toHaveLength(2)
  })

  it("throws when a removed item is changed", async () => {
    const { bar } = await setup()
    const item = await StatusItem.create({ id: "a", bar })
    item.remove()
    expect(() => item.setLabel("x")).toThrow(/removed/)
    item.remove() // second remove is harmless
  })

  it("delivers clicks to the right item", async () => {
    const { fake, bar } = await setup()
    const a = await StatusItem.create({ id: "a", bar })
    const b = await StatusItem.create({ id: "b", bar })
    const clicks: string[] = []
    a.on("click", ({ button }) => clicks.push(`a:${button}`))
    b.on("click", ({ button }) => clicks.push(`b:${button}`))
    fake.send({ type: "item.click", id: "b", button: "right" })
    fake.send({ type: "item.click", id: "a", button: "left" })
    fake.send({ type: "item.click", id: "unknown", button: "left" })
    await until(() => clicks.length === 2)
    expect(clicks).toEqual(["b:right", "a:left"])
  })

  it("routes bar errors to the item, or to the connection when nobody listens", async () => {
    const { fake, bar } = await setup()
    const item = await StatusItem.create({ id: "a", bar })
    const other = await StatusItem.create({ id: "other", bar })
    const itemErrors: BarError[] = []
    const barErrors: BarError[] = []
    item.on("error", (e) => itemErrors.push(e))
    bar.on("error", (e) => barErrors.push(e))
    const [, create, otherCreate] = (await fake.messages(3)) as Array<{ requestId: string }>
    expect(other.id).toBe("other")
    fake.send({
      type: "error",
      requestId: create!.requestId,
      code: "limit_reached",
      message: "full",
    })
    fake.send({
      type: "error",
      requestId: otherCreate!.requestId,
      code: "invalid_icon",
      message: "bad",
    })
    fake.send({ type: "error", requestId: null, code: "unknown_type", message: "?" })
    await until(() => itemErrors.length === 1 && barErrors.length === 2)
    expect(itemErrors[0]).toMatchObject({ code: "limit_reached" })
    expect(barErrors.map((e) => e.code)).toEqual(["invalid_icon", "unknown_type"])
  })

  it("does nothing, without throwing, after the connection closed", async () => {
    const { fake, bar } = await setup()
    const item = await StatusItem.create({ id: "a", bar })
    let closed = 0
    bar.on("close", () => closed++)
    fake.closeClient()
    await until(() => bar.closed)
    expect(closed).toBe(1)
    expect(() => item.setLabel("late")).not.toThrow()
  })

  it("forgets an item whose create the bar refused, so the id can be used again", async () => {
    const { fake, bar } = await setup()
    const item = await StatusItem.create({ id: "a", bar })
    item.on("error", () => {})
    const [, create] = (await fake.messages(2)) as Array<{ requestId: string }>
    fake.send({
      type: "error",
      requestId: create!.requestId,
      code: "limit_reached",
      message: "full",
    })
    await until(() => !bar.items.has("a"))
    await expect(StatusItem.create({ id: "a", bar })).resolves.toBeInstanceOf(StatusItem)
  })

  it("keeps handling messages after a click listener throws", async () => {
    const { fake, bar } = await setup()
    const a = await StatusItem.create({ id: "a", bar })
    const b = await StatusItem.create({ id: "b", bar })
    const seen: string[] = []
    a.on("click", () => {
      throw new Error("listener bug")
    })
    b.on("click", () => seen.push("b"))
    // Catch the rethrown error so the test runner does not see it as unhandled.
    const crashes: unknown[] = []
    const onCrash = (e: unknown) => crashes.push(e)
    process.once("uncaughtException", onCrash)
    // Both clicks in one write, so they arrive in one chunk.
    const both = Buffer.concat([
      encodeFrame({ type: "item.click", id: "a", button: "left" }),
      encodeFrame({ type: "item.click", id: "b", button: "left" }),
    ])
    fake.sendRaw(both)
    await until(() => seen.length === 1 && crashes.length === 1)
    process.off("uncaughtException", onCrash)
  })

  it("concurrent creates share one connection", async () => {
    fake = new FakeBar()
    await fake.listen()
    const bar = await StatusBar.connect({ pipeName: fake.pipeName })
    bars.push(bar)
    const items = await Promise.all([
      StatusItem.create({ id: "a", bar }),
      StatusItem.create({ id: "b", bar }),
    ])
    expect(items.map((i) => i.id)).toEqual(["a", "b"])
  })
})
