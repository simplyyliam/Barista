// The installer and the updater compare versions, so every place that carries the app version
// must say the same thing. A release that forgot one would never be offered as an update.
import { readFileSync } from "node:fs"
import { join } from "node:path"
import { fileURLToPath } from "node:url"
import { describe, expect, it } from "vitest"

const root = fileURLToPath(new URL("..", import.meta.url))
const read = (path: string) => readFileSync(join(root, path), "utf8")

describe("app version", () => {
  it("is the same in package.json, the bar, Cargo.toml and tauri.conf.json", () => {
    const cargo = /^version = "([^"]+)"/m.exec(read("apps/bar/src-tauri/Cargo.toml"))?.[1]
    const versions = {
      "package.json": JSON.parse(read("package.json")).version,
      "apps/bar/package.json": JSON.parse(read("apps/bar/package.json")).version,
      "Cargo.toml": cargo,
      "tauri.conf.json": JSON.parse(read("apps/bar/src-tauri/tauri.conf.json")).version,
    }
    expect(new Set(Object.values(versions)).size, JSON.stringify(versions)).toBe(1)
  })
})
