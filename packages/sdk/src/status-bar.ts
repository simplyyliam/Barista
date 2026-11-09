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
  /** How long to wait for the bar to answer, in milliseconds. Default 5000. */
  timeoutMs?: number
  /** Another pipe to connect to. For tests; apps never need it. */
  pipeName?: string
}

type Listener<T> = (event: T) => void

/**
 * Your app's connection to the Barista bar. Most apps never touch it directly: creating a
 * {@link StatusItem} connects automatically. Use it to know when the bar goes away.
 *
 * @example
 * ```ts
 * const bar = await BA.connect({ appName: "Clock" })
 * bar.on("close", () => process.exit(0))
 * ```
 */
export class StatusBar {
  /** Identifies this connection to the bar, for example `"s3"`. */
  readonly sessionId: string
  /** The version of the bar you are connected to. */
  readonly barVersion: string

  readonly #socket: net.Socket
  readonly #decoder: FrameDecoder
  readonly #errorListeners = new Set<Listener<BarError>>()
  readonly #closeListeners = new Set<Listener<void>>()
  #closed = false
  #requestCounter = 0
  /** @internal items by id, so clicks and errors find their item */
  readonly items = new Map<string, StatusItem>()

  private constructor(
    socket: net.Socket,
    decoder: FrameDecoder,
    welcome: Extract<BarMessage, { type: "welcome" }>,
  ) {
    this.#socket = socket
    this.sessionId = welcome.sessionId
    this.barVersion = welcome.barVersion
    // Carry over frames that arrived in the same chunk as the welcome.
    this.#decoder = decoder
    socket.on("data", (chunk) => this.#onData(chunk))
    socket.on("error", () => {}) // a broken pipe also ends in "close"
    socket.on("close", () => this.#onClose())
  }

  /**
   * Connects to the bar and introduces your app (`hello`). Rejects when the bar is not
   * running or does not accept the connection.
   *
   * @example
   * ```ts
   * import * as BA from "@barista/sdk"
   * const bar = await BA.StatusBar.connect({ appName: "Clock" })
   * ```
   */
  static connect(options: ConnectOptions = {}): Promise<StatusBar> {
    return new Promise((resolve, reject) => {
      const socket = net.connect(options.pipeName ?? PIPE_NAME)
      const decoder = new FrameDecoder()
      let done = false
      const finish = (error?: Error, bar?: StatusBar) => {
        if (done) return
        done = true
        clearTimeout(timer)
        if (error) {
          socket.destroy()
          reject(error)
        } else if (bar) {
          resolve(bar)
        }
      }
      const timer = setTimeout(
        () => finish(new Error("The Barista bar did not answer in time")),
        options.timeoutMs ?? 5000,
      )
      socket.once("connect", () => {
        const hello: ClientMessage = {
          type: "hello",
          protocolVersion: PROTOCOL_VERSION,
          appName: options.appName ?? defaultAppName(),
          pid: process.pid,
          sdk: { name: SDK_NAME, version: SDK_VERSION },
        }
        socket.write(encodeFrame(hello))
      })
      socket.on("data", function onData(chunk) {
        if (done) return
        let messages: unknown[]
        try {
          messages = decoder.push(chunk)
        } catch (error) {
          return finish(error as Error)
        }
        const first = messages[0] as BarMessage | undefined
        if (!first) return
        if (first.type === "welcome") {
          socket.off("data", onData)
          socket.off("error", onError)
          socket.off("close", onClose)
          const bar = new StatusBar(socket, decoder, first)
          finish(undefined, bar)
          // Anything after the welcome in the same chunk belongs to the bar object.
          for (const message of messages.slice(1)) bar.#handle(message as BarMessage)
        } else if (first.type === "error") {
          finish(new BarError(first.code, first.message))
        } else {
          finish(new Error("The bar answered hello with something unexpected"))
        }
      })
      const onError = (error: NodeJS.ErrnoException) => {
        const missing = error.code === "ENOENT" || error.code === "ECONNREFUSED"
        finish(
          missing
            ? new Error("Could not reach the Barista bar. Is it running?", { cause: error })
            : error,
        )
      }
      const onClose = () => finish(new Error("The bar closed the connection"))
      socket.on("error", onError)
      socket.on("close", onClose)
    })
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
   * Called once when the connection to the bar ends (the bar quit or crashed). After that,
   * item calls do nothing.
   *
   * @example
   * ```ts
   * bar.on("close", () => console.log("bar is gone"))
   * ```
   */
  on(event: "close", listener: Listener<void>): this
  on(event: "error" | "close", listener: Listener<never>): this {
    if (event === "error") this.#errorListeners.add(listener as Listener<BarError>)
    else this.#closeListeners.add(listener as Listener<void>)
    return this
  }

  /** Stops a listener added with {@link StatusBar.on}. */
  off(event: "error" | "close", listener: Listener<never>): this {
    if (event === "error") this.#errorListeners.delete(listener as Listener<BarError>)
    else this.#closeListeners.delete(listener as Listener<void>)
    return this
  }

  /**
   * Disconnects. The bar removes all of your items.
   *
   * @example
   * ```ts
   * process.on("SIGINT", () => bar.close())
   * ```
   */
  close(): void {
    this.#socket.end()
  }

  /** True after the connection has ended. */
  get closed(): boolean {
    return this.#closed
  }

  /** @internal Sends a message. Does nothing once the connection is closed. */
  send(message: ClientMessage): void {
    if (this.#closed) return
    this.#socket.write(encodeFrame(message))
  }

  /** @internal A request id that leads back to an item: `"<itemId>#<n>"`. */
  requestIdFor(itemId: string): string {
    return `${itemId}#${++this.#requestCounter}`
  }

  #onData(chunk: Buffer): void {
    let messages: unknown[]
    try {
      messages = this.#decoder.push(chunk)
    } catch (error) {
      this.#emitError(new BarError("invalid_message", (error as Error).message))
      this.#socket.destroy()
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
      case "item.click":
        this.items.get(message.id)?.emitClick(message.button)
        break
      case "error": {
        const error = new BarError(message.code, message.message)
        const itemId = message.requestId?.split("#")[0]
        const item = itemId === undefined ? undefined : this.items.get(itemId)
        if (!(item?.emitError(error) ?? false)) this.#emitError(error)
        // The bar refused the create itself: the item does not exist there, so forget it.
        if (item && itemId !== undefined && message.requestId === item.createRequestId) {
          this.items.delete(itemId)
        }
        break
      }
      default:
        break // welcome again, debug.items, or something from a newer bar: ignore
    }
  }

  #emitError(error: BarError): void {
    if (this.#errorListeners.size === 0) {
      console.warn(`[barista] ${error.code}: ${error.message}`)
      return
    }
    for (const listener of this.#errorListeners) listener(error)
  }

  #onClose(): void {
    if (this.#closed) return
    this.#closed = true
    for (const listener of this.#closeListeners) listener()
  }
}

/** The running script's name without the extension, or a generic name in a REPL. */
function defaultAppName(): string {
  const script = process.argv[1]
  return script ? basename(script, extname(script)) || "Node app" : "Node app"
}

let defaultBar: Promise<StatusBar> | undefined

/**
 * Connects to the bar once and shares that connection with every {@link StatusItem} you
 * create afterwards. Call it first if you want to set `appName` or react to `close`;
 * otherwise `StatusItem.create` calls it for you. If the connection ended, the next call
 * connects again.
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
  return attempt
}
