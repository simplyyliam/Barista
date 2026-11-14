import { readFileSync } from "node:fs"
import { describe, expect, it } from "vitest"
import * as BA from "./index.js"

describe("@baristabar/sdk", () => {
  it("exports the protocol version generated from Rust", () => {
    expect(BA.PROTOCOL_VERSION).toMatch(/^\d+\.\d+$/)
  })

  it("SDK_VERSION matches package.json", () => {
    const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"))
    expect(BA.SDK_VERSION).toBe(pkg.version)
    expect(BA.SDK_NAME).toBe(pkg.name)
  })
})
