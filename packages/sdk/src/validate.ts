// The same rules the bar enforces (limits come from the Rust protocol crate through
// protocol.ts), checked early so a mistake throws at the call that caused it instead of
// coming back later as an error message.

import { MAX_ICON_BYTES, MAX_ID_LEN, MAX_LABEL_LEN, MAX_TOOLTIP_LEN } from "./protocol.js"

/** Item ids: 1 to 64 characters from `a-z 0-9 - _ .` */
export function assertItemId(id: string): void {
  if (typeof id !== "string" || id.length === 0 || id.length > MAX_ID_LEN) {
    throw new TypeError(`Item id must be a string of 1 to ${MAX_ID_LEN} characters`)
  }
  if (!/^[a-z0-9\-_.]+$/.test(id)) {
    throw new TypeError(`Item id ${JSON.stringify(id)} may only use a-z, 0-9, "-", "_" and "."`)
  }
}

function assertText(field: string, value: string, max: number): void {
  if (typeof value !== "string") throw new TypeError(`${field} must be a string`)
  if ([...value].length > max) throw new TypeError(`${field} is longer than ${max} characters`)
  // eslint-disable-next-line no-control-regex
  if (/[\u0000-\u001f\u007f-\u009f]/.test(value)) {
    throw new TypeError(`${field} contains control characters`)
  }
}

export const assertLabel = (value: string) => assertText("label", value, MAX_LABEL_LEN)
export const assertTooltip = (value: string) => assertText("tooltip", value, MAX_TOOLTIP_LEN)

// Built from text so the file holds no invisible character.
const BOM = new RegExp("^\uFEFF")
const PNG_MAGIC = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]

/**
 * Accepts the icon formats the SDK allows and returns a `data:` URL for the wire.
 * Bytes are sniffed: a PNG starts with the PNG signature, an SVG with `<svg` or `<?xml`.
 * A string must already be a `data:` URL. An empty string clears the icon.
 * @internal
 */
export function toIconDataUrl(icon: Uint8Array | string): string {
  if (typeof icon === "string") {
    if (icon !== "" && !/^data:image\/(png|svg\+xml);base64,/.test(icon)) {
      throw new TypeError("A string icon must be a base64 data: URL of a PNG or SVG")
    }
    return icon
  }
  if (!(icon instanceof Uint8Array)) throw new TypeError("icon must be bytes or a data: URL")
  if (icon.length > MAX_ICON_BYTES) {
    throw new TypeError(`Icon is ${icon.length} bytes, the limit is ${MAX_ICON_BYTES}`)
  }
  const bytes = Buffer.from(icon.buffer, icon.byteOffset, icon.byteLength)
  const isPng = PNG_MAGIC.every((byte, i) => bytes[i] === byte)
  const head = bytes.subarray(0, 512).toString("utf8").replace(BOM, "").trimStart()
  const isSvg = head.startsWith("<svg") || head.startsWith("<?xml") || head.startsWith("<!--")
  if (!isPng && !isSvg) throw new TypeError("Icon must be a PNG or an SVG image")
  return `data:${isPng ? "image/png" : "image/svg+xml"};base64,${bytes.toString("base64")}`
}
