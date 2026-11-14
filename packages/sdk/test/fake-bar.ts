// A stand-in for the Barista bar on a real named pipe, shared by the SDK tests. It records what
// clients send and lets a test answer. Make a second one on the same `pipeName` to play a bar
// that was restarted.
import net from "node:net"
import { randomUUID } from "node:crypto"
import { FrameDecoder, encodeFrame } from "../src/frame.js"
import { PROTOCOL_VERSION, type BarMessage, type ClientMessage } from "../src/protocol.js"

export class FakeBar {
  readonly pipeName: string
  readonly received: ClientMessage[] = []
  /** Connections accepted so far. */
  connections = 0
  readonly #server: net.Server
  #socket?: net.Socket
  #waiters: Array<() => void> = []
  /** What to answer to `hello`. */
  onHello: (hello: ClientMessage) => BarMessage | null = () => ({
    type: "welcome",
    sessionId: `s${this.connections}`,
    barVersion: "9.9.9",
    protocolVersion: PROTOCOL_VERSION,
  })

  constructor(pipeName = `\\\\.\\pipe\\barista-test-${randomUUID()}`) {
    this.pipeName = pipeName
    this.#server = net.createServer((socket) => {
      this.#socket = socket
      this.connections++
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

  /** Hangs up on the client but keeps listening (like a bar that dropped one connection). */
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

  /** Resolves with the first message that matches, waiting for it if needed. */
  async next(match: (message: ClientMessage) => boolean, from = 0): Promise<ClientMessage> {
    for (;;) {
      const found = this.received.slice(from).find(match)
      if (found) return found
      await new Promise<void>((resolve) => this.#waiters.push(resolve))
    }
  }

  close(): void {
    this.#socket?.destroy()
    this.#server.close()
  }

  /** Like `close`, and resolves once the pipe name is free, so a new bar can take it. */
  async stop(): Promise<void> {
    this.#socket?.destroy()
    await new Promise<void>((resolve) => this.#server.close(() => resolve()))
  }
}
