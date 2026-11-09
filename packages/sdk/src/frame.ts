import { MAX_MESSAGE_BYTES } from "./protocol.js"

/**
 * Turns a message into one frame: 4 bytes (little-endian) with the size of the JSON, then the
 * JSON itself. This is what goes over the pipe.
 * @internal
 */
export function encodeFrame(message: unknown): Buffer {
  const json = Buffer.from(JSON.stringify(message), "utf8")
  if (json.length > MAX_MESSAGE_BYTES) {
    throw new Error(`Message is ${json.length} bytes, the limit is ${MAX_MESSAGE_BYTES}`)
  }
  const frame = Buffer.allocUnsafe(4 + json.length)
  frame.writeUInt32LE(json.length, 0)
  json.copy(frame, 4)
  return frame
}

/**
 * Collects chunks from the socket and hands back whole messages. A chunk may hold half a
 * message or several, so it keeps the leftover between calls.
 * @internal
 */
export class FrameDecoder {
  #buffer: Buffer = Buffer.alloc(0)

  /** Adds a chunk and returns every message that is now complete. Throws on a bad length. */
  push(chunk: Buffer): unknown[] {
    this.#buffer = this.#buffer.length === 0 ? chunk : Buffer.concat([this.#buffer, chunk])
    const messages: unknown[] = []
    while (this.#buffer.length >= 4) {
      const length = this.#buffer.readUInt32LE(0)
      if (length === 0 || length > MAX_MESSAGE_BYTES) {
        throw new Error(`The bar sent a frame of ${length} bytes, which is not allowed`)
      }
      if (this.#buffer.length < 4 + length) break
      const json = this.#buffer.subarray(4, 4 + length).toString("utf8")
      this.#buffer = this.#buffer.subarray(4 + length)
      messages.push(JSON.parse(json))
    }
    return messages
  }
}
