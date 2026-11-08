// Guards the website setup: static export stays on and the two placeholder docs pages exist.
import { readFileSync } from "node:fs"
import { fileURLToPath } from "node:url"
import { describe, expect, it } from "vitest"

const read = (path: string) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), "utf8")

describe("website", () => {
  it("is a static export", () => {
    expect(read("./next.config.mjs")).toMatch(/output:\s*"export"/)
  })

  it.each([
    ["index.mdx", "Getting started"],
    ["protocol.mdx", "Protocol"],
  ])("has the %s placeholder page", (file, title) => {
    const page = read(`./content/docs/${file}`)
    expect(page).toContain(`title: ${title}`)
    expect(page).toContain("Coming in v0.10.")
  })
})
