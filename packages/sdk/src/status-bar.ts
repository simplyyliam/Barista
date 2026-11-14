import net from "node:net"
import { basename, extname } from "node:path"
import { FrameDecoder, encodeFrame } from "./frame.js"
import {
  PROTOCOL_VERSION,
  type BarMessage,
  type ClientMessage,
  type ErrorCode,
} from "./protocol.js"
import type { StatusItem } from "./status-item.js"
import { SDK_NAME, SDK_VERSION } from "./version.js"

/** The pipe the Barista bar listens on. */
export const PIPE_NAME = "\\\\.\\pipe\\barista"

/** An error the bar sent back, with its machine-readable `code`. */
export class BarError extends Error {
  /** What went wrong, for example `"invalid_icon"` or `"limit_reached"`. */
  readonly code: ErrorCode
  constructor(code: ErrorCode, message: string) {
    super(message)
    this.name = "BarError"
    this.code = code
  }
}

/** Options for {@link connect}. */
export interface ConnectOptions {
  /**
   * Your app's name, shown by the bar (in tooltips and debug tools).
   * Defaults to the name of the script that is running.
   */
  appName?: string
  /**
   * How long one connection attempt waits for the bar to answer, in milliseconds. Default 5000.
   * A bar that does not answer is retried, not reported.
   */
  timeoutMs?: number
  /** Another pipe to connect to. For tests; apps never need it. */
  pipeName?: string
}

type Listener<T> = (event: T) => void
type Event = "error" | "close" | "quit" | "available" | "unavailable"

/**
 * Waits before reconnect attempt number `attempt` (0 is the first retry): 100, 200, 400 ms, then
 * every half second, each a little random so many apps do not knock at the same moment. The cap
 * is what brings items back soon after the bar starts (at most half a second plus one connection).
 * A failed open of a missing pipe costs next to nothing, so two a second while the bar is away
 * is fine.
 * @internal
 */
export function retryDelay(attempt: number, random: () => number = Math.random): number {
  const steps = [100, 200, 400]
  const base = steps[attempt] ?? 500
  return Math.min(500, Math.round(base * (0.8 + random() * 0.4)))
}

/** Outcome of one connection attempt. A refused `hello` rejects instead. */
type Attempt =
  | { kind: "retry" }
  | {
      kind: "connected"
      socket: net.Socket
      decoder: FrameDecoder
      welcome: Extract<BarMessage, { type: "welcome" }>
      /** Messages that arrived in the same chunk as the welcome. */
      rest: BarMessage[]
    }

/** The connection most recently made by {@link connect}, for `isAvailable`. */
let sharedBar: StatusBar | undefined

/**
 * Your app's connection to the Barista bar. It lives as long as your app: when the bar is not
 * running it waits quietly, and when the bar goes away or restarts it reconnects by itself and
 * puts every {@link StatusItem} back, with its menu and popover and your callbacks.
 *
 * @example
 * ```ts
 * const bar = await BA.connect({ appName: "Clock" })
 * bar.on("unavailable", () => console.log("bar is gone, waiting for it"))
 * bar.on("available", () => console.log("bar is here"))
 * ```
 */
export class StatusBar {
  readonly #options: ConnectOptions
  readonly #listeners: Record<Event, Set<Listener<never>>> = {
    error: new Set(),
    close: new Set(),
    quit: new Set(),
    available: new Set(),
    unavailable: new Set(),
  }
  #state: "connecting" | "connected" | "waiting" | "closed" = "connecting"
  #socket: net.Socket | undefined
  #sessionId: string | undefined
  #barVersion: string | undefined
  #attempts = 0
  #retryTimer: NodeJS.Timeout | undefined
  #closeEmitted = false
  #requestCounter = 0
  /** @internal items by id, so clicks and errors find their item, and a reconnect finds them all */
  readonly items = new Map<string, StatusItem>()

  private constructor(options: ConnectOptions) {
    this.#options = options
  }

  /**
   * Identifies this connection to the bar, for example `"s3"`. `undefined` while the bar is
   * not there; it changes after every reconnect.
   */
  get sessionId(): string | undefined {
    return this.#sessionId
  }

  /** The version of the bar you are connected to, or `undefined` while it is not there. */
  get barVersion(): string | undefined {
    return this.#barVersion
  }

  /**
   * True while you are connected to the bar. Items you create while it is false appear as soon
   * as it comes back.
   *
   * @example
   * ```ts
   * if (!bar.available) console.log("Barista is not running, waiting for it")
   * ```
   */
  get available(): boolean {
    return this.#state === "connected"
  }

  /** True once the connection is over for good (you called `close()`, or the bar refused you). */
  get closed(): boolean {
    return this.#state === "closed"
  }

  /**
   * Connects to the bar and introduces your app (`hello`). A bar that is not running is not an
   * error: this resolves anyway, {@link StatusBar.available} is `false`, and the connection keeps
   * trying quietly until the bar shows up. It rejects only when the bar refuses your app for
   * good, for example `"version_mismatch"`.
   *
   * @example
   * ```ts
   * import * as BA from "@barista/sdk"
   * const bar = await BA.StatusBar.connect({ appName: "Clock" })
   * ```
   */
  static async connect(options: ConnectOptions = {}): Promise<StatusBar> {
    const bar = new StatusBar(options)
    try {
      await bar.#attempt()
    } catch (error) {
      bar.#closeForGood(false)
      throw error
    }
    return bar
  }

  /**
   * Is the Barista bar running right now? Uses your connection when you have one, otherwise
   * peeks at the bar without joining it. Handy to pick Barista or a fallback.
   *
   * @example
   * ```ts
   * if (await BA.StatusBar.isAvailable()) {
   *   await BA.StatusItem.create({ id: "mail", label: "3" })
   * } else {
   *   showTrayIconInstead()
   * }
   * ```
   */
  static async isAvailable(options: Pick<ConnectOptions, "pipeName" | "timeoutMs"> = {}): Promise<boolean> {
    if (options.pipeName === undefined && sharedBar?.available) return true
    return probe(options.pipeName ?? PIPE_NAME, options.timeoutMs ?? 1000)
  }

  /**
   * Listens for errors the bar reports that do not belong to one item, and for items you
   * did not listen on. Without a listener they are printed with `console.warn`.
   *
   * @example
   * ```ts
   * bar.on("error", (error) => console.error(error.code, error.message))
   * ```
   */
  on(event: "error", listener: Listener<BarError>): this
  /**
   * Called once when the connection is over for good: after `bar.close()`, or when the bar
   * refuses your app. It is not called when the bar quits or restarts (that is `unavailable`).
   *
   * @example
   * ```ts
   * bar.on("close", () => console.log("done with the bar"))
   * ```
   */
  on(event: "close", listener: Listener<void>): this
  /**
   * Called when the user picks a `role: "quit"` menu entry that has no `click` of its own.
   * Your items are already removed. Without a listener the SDK closes the connection and
   * ends the process (`process.exit(0)`); with one, quitting is up to you.
   *
   * @example
   * ```ts
   * bar.on("quit", async () => {
   *   await saveState()
   *   bar.close()
   *   process.exit(0)
   * })
   * ```
   */
  on(event: "quit", listener: Listener<void>): this
  /**
   * Called each time the bar comes back after it was away, with your items already back in it.
   * The very first connection happens before `connect()` returns, so check `bar.available` for
   * that one.
   *
   * @example
   * ```ts
   * if (bar.available) console.log("connected")
   * bar.on("available", () => console.log(`connected to bar ${bar.barVersion}`))
   * ```
   */
  on(event: "available", listener: Listener<void>): this
  /**
   * Called when the bar goes away (quit, crash, restart). Your items stay yours: they are put
   * back by themselves when the bar returns. Open menus and popovers of yours are closed.
   *
   * @example
   * ```ts
   * bar.on("unavailable", () => console.log("waiting for the bar"))
   * ```
   */
  on(event: "unavailable", listener: Listener<void>): this
  on(event: Event, listener: Listener<never>): this {
    this.#listeners[event].add(listener)
    return this
  }

  /**
   * Stops a listener added with {@link StatusBar.on}.
   *
   * @example
   * ```ts
   * const onGone = () => console.log("bar is gone")
   * bar.on("unavailable", onGone)
   * bar.off("unavailable", onGone)
   * ```
   */
  off(event: Event, listener: Listener<never>): this {
    this.#listeners[event].delete(listener)
    return this
  }

  /** @internal The user asked the app to quit from a menu (`role: "quit"`). */
  quit(): void {
    for (const item of [...this.items.values()]) item.remove()
    if (this.#listeners.quit.size > 0) {
      this.#emit("quit")
      return
    }
    this.close()
    // Give the socket a moment to send the removes before the process goes.
    setTimeout(() => process.exit(0), 50)
  }

  /**
   * Disconnects for good: the bar removes all of your items and the connection does not
   * come back. Call it when your app is done with the bar.
   *
   * @example
   * ```ts
   * process.on("SIGINT", () => bar.close())
   * ```
   */
  close(): void {
    this.#closeForGood(true)
  }

  /** @internal Sends a message. Does nothing while the bar is not there. */
  send(message: ClientMessage): void {
    if (this.#state !== "connected") return
    this.#socket?.write(encodeFrame(message))
  }

  /** @internal A request id that leads back to an item: `"<itemId>#<n>"`. */
  requestIdFor(itemId: string): string {
    return `${itemId}#${++this.#requestCounter}`
  }

  /** One try at reaching the bar. Rejects on a refused hello, otherwise settles into a state. */
  async #attempt(): Promise<void> {
    if (this.#state === "closed") return
    this.#state = "connecting"
    const result = await this.#tryConnect()
    if (this.closed) {
      if (result.kind === "connected") result.socket.destroy() // closed while we were connecting
      return
    }
    if (result.kind === "retry") {
      this.#state = "waiting"
      this.#scheduleRetry()
      return
    }
    this.#adopt(result)
  }

  #tryConnect(): Promise<Attempt> {
    return new Promise((resolve, reject) => {
      const socket = net.connect(this.#options.pipeName ?? PIPE_NAME)
      const decoder = new FrameDecoder()
      let done = false
      const finish = (settle: () => void) => {
        if (done) return
        done = true
        clearTimeout(timer)
        socket.off("data", onData)
        settle()
      }
      // Anything that goes wrong before the welcome (no pipe, a busy pipe, a bar that does
      // not answer) just means "try again later".
      const retry = () =>
        finish(() => {
          socket.destroy()
          resolve({ kind: "retry" })
        })
      const timer = setTimeout(retry, this.#options.timeoutMs ?? 5000)
      const onData = (chunk: Buffer) => {
        let messages: BarMessage[]
        try {
          messages = decoder.push(chunk) as BarMessage[]
        } catch {
          return retry()
        }
        const first = messages[0]
        if (!first) return
        if (first.type === "welcome") {
          finish(() => resolve({ kind: "connected", socket, decoder, welcome: first, rest: messages.slice(1) }))
        } else if (first.type === "error") {
          // Asking again with the same hello would be refused again: do not retry.
          finish(() => {
            socket.destroy()
            reject(new BarError(first.code, first.message))
          })
        } else {
          retry()
        }
      }
      socket.once("connect", () => {
        const hello: ClientMessage = {
          type: "hello",
          protocolVersion: PROTOCOL_VERSION,
          appName: this.#options.appName ?? defaultAppName(),
          pid: process.pid,
          sdk: { name: SDK_NAME, version: SDK_VERSION },
        }
        socket.write(encodeFrame(hello))
      })
      socket.on("data", onData)
      socket.on("error", retry)
      socket.on("close", retry)
    })
  }

  /** Takes over a welcomed socket: replays the items, then starts listening. */
  #adopt(attempt: Extract<Attempt, { kind: "connected" }>): void {
    const { socket, decoder, welcome, rest } = attempt
    this.#socket = socket
    this.#sessionId = welcome.sessionId
    this.#barVersion = welcome.barVersion
    this.#state = "connected"
    this.#attempts = 0
    socket.on("data", (chunk) => this.#onData(socket, decoder, chunk))
    socket.on("close", () => this.#onSocketClose(socket))
    // The handshake listener is gone; keep ignoring errors (a broken pipe ends in "close").
    socket.on("error", () => {})
    // After a restart the bar knows nothing about us: say it all again, in creation order.
    for (const item of this.items.values()) {
      try {
        item.replay()
      } catch (error) {
        this.#emitError(new BarError("invalid_message", (error as Error).message))
      }
    }
    for (const message of rest) this.#handle(message)
    this.#emit("available")
  }

  #onSocketClose(socket: net.Socket): void {
    if (socket !== this.#socket) return
    this.#socket = undefined
    this.#sessionId = undefined
    this.#barVersion = undefined
    if (this.#state === "closed") {
      this.#emitClosed()
      return
    }
    // The bar quit, crashed or cut us off: wait for it, and come back.
    this.#state = "waiting"
    this.#suspendItems()
    this.#emit("unavailable")
    this.#scheduleRetry()
  }

  #scheduleRetry(): void {
    if (this.#state === "closed") return
    // Not unref'd: an app that only shows an item must keep running while the bar is away.
    this.#retryTimer = setTimeout(() => {
      this.#retryTimer = undefined
      this.#attempt().catch((error: unknown) => {
        this.#emitError(error as BarError)
        this.#closeForGood(false)
      })
    }, retryDelay(this.#attempts++))
  }

  /** The connection is over and does not come back. `bye` tells the bar it was on purpose. */
  #closeForGood(sayBye: boolean): void {
    if (this.#state === "closed") return
    const socket = this.#socket
    this.#state = "closed"
    clearTimeout(this.#retryTimer)
    this.#retryTimer = undefined
    this.#suspendItems()
    if (!socket || socket.destroyed) {
      queueMicrotask(() => this.#emitClosed())
      return
    }
    if (sayBye) {
      try {
        socket.write(encodeFrame({ type: "bye" } satisfies ClientMessage))
      } catch {
        // nothing to say if the socket is already broken
      }
    }
    // "close" follows when the bar hangs up. If it never does, do not wait for it.
    socket.end()
    setTimeout(() => socket.destroy(), 1000).unref()
  }

  /** Items close their own menu and popover when the bar is gone. */
  #suspendItems(): void {
    for (const item of this.items.values()) item.suspend()
  }

  #onData(socket: net.Socket, decoder: FrameDecoder, chunk: Buffer): void {
    if (socket !== this.#socket) return
    let messages: unknown[]
    try {
      messages = decoder.push(chunk)
    } catch (error) {
      this.#emitError(new BarError("invalid_message", (error as Error).message))
      socket.destroy() // ends in "close", which reconnects
      return
    }
    for (const message of messages) this.#handle(message as BarMessage)
  }

  /** One bad listener must not stop the messages that arrived in the same chunk. */
  #handle(message: BarMessage): void {
    try {
      this.#dispatch(message)
    } catch (error) {
      // Still crash like any uncaught error would, but only after the other messages ran.
      queueMicrotask(() => {
        throw error
      })
    }
  }

  #dispatch(message: BarMessage): void {
    switch (message.type) {
      case "ping":
        this.send({ type: "pong", seq: message.seq })
        break
      case "shutdown":
        // Quit and unresponsive mean the bar comes back: the "close" that follows starts the
        // waiting. A bar that cut us off for noise would refuse the same messages again.
        if (message.reason === "too_many_errors") this.#closeForGood(false)
        break
      case "item.click":
        this.items.get(message.id)?.emitClick(message.button, message.anchor)
        break
      case "popover.show":
        void this.items.get(message.id)?.emitPopoverShow({
          ...message.rect,
          scale: message.scale ?? 1,
          monitor: message.monitor,
          tone: message.tone,
        })
        break
      case "popover.hide":
        this.items.get(message.id)?.emitPopoverHide()
        break
      case "menu.opened":
        this.items.get(message.id)?.emitMenuOpened()
        break
      case "menu.closed":
        this.items.get(message.id)?.emitMenuClosed()
        break
      case "menu.click":
        this.items.get(message.id)?.emitMenuClick(message.menuItemId, message.checked ?? undefined)
        break
      case "error": {
        const error = new BarError(message.code, message.message)
        const itemId = message.requestId?.split("#")[0]
        const item = itemId === undefined ? undefined : this.items.get(itemId)
        if (!(item?.emitError(error) ?? false)) this.#emitError(error)
        // The bar refused the create itself: the item does not exist there, so forget it.
        if (item && itemId !== undefined && message.requestId === item.createRequestId) {
          item.markGone() // later calls throw here, instead of earning strikes at the bar
          this.items.delete(itemId)
        }
        break
      }
      default:
        break // welcome again, debug.items, or something from a newer bar: ignore
    }
  }

  #emit(event: "close" | "quit" | "available" | "unavailable"): void {
    for (const listener of this.#listeners[event]) (listener as Listener<void>)()
  }

  #emitClosed(): void {
    if (this.#closeEmitted) return
    this.#closeEmitted = true
    this.#emit("close")
  }

  #emitError(error: BarError): void {
    const listeners = this.#listeners.error
    if (listeners.size === 0) {
      console.warn(`[barista] ${error.code}: ${error.message}`)
      return
    }
    for (const listener of listeners) (listener as Listener<BarError>)(error)
  }
}

/** Opens the pipe and closes it again, to see whether a bar is listening. */
function probe(pipeName: string, timeoutMs: number): Promise<boolean> {
  return new Promise((resolve) => {
    const socket = net.connect(pipeName)
    const finish = (up: boolean) => {
      clearTimeout(timer)
      socket.destroy()
      resolve(up)
    }
    const timer = setTimeout(() => finish(false), timeoutMs)
    socket.once("connect", () => finish(true))
    socket.on("error", () => finish(false))
  })
}

/** The running script's name without the extension, or a generic name in a REPL. */
function defaultAppName(): string {
  const script = process.argv[1]
  return script ? basename(script, extname(script)) || "Node app" : "Node app"
}

let defaultBar: Promise<StatusBar> | undefined

/**
 * Connects to the bar once and shares that connection with every {@link StatusItem} you
 * create afterwards. Call it first if you want to set `appName` or react to `available` and
 * `unavailable`; otherwise `StatusItem.create` calls it for you. It resolves even if the bar is
 * not running yet: your items appear when it starts.
 *
 * @example
 * ```ts
 * import * as BA from "@barista/sdk"
 * await BA.connect({ appName: "Clock" })
 * const item = await BA.StatusItem.create({ id: "time", label: "12:00" })
 * ```
 */
export async function connect(options: ConnectOptions = {}): Promise<StatusBar> {
  const existing = defaultBar
  if (existing) {
    const bar = await existing.catch(() => undefined)
    if (bar && !bar.closed) return bar
    // While we waited, another caller may have started a new connection: share that one.
    if (defaultBar !== existing) return connect(options)
  }
  const attempt = StatusBar.connect(options)
  defaultBar = attempt
  // A failed attempt must not be remembered, or every later call would fail the same way.
  attempt.catch(() => {
    if (defaultBar === attempt) defaultBar = undefined
  })
  sharedBar = await attempt
  return sharedBar
}
