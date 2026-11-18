// Guards the website setup: static export stays on and the two placeholder docs pages exist.
import { readdirSync, readFileSync } from "node:fs"
import { fileURLToPath } from "node:url"
import { describe, expect, it } from "vitest"

const read = (path: string) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), "utf8")

describe("website", () => {
  it("is a static export", () => {
    expect(read("./next.config.mjs")).toMatch(/output:\s*"export"/)
  })

  it("never calls Barista open source (closed source, freemium)", () => {
    const dir = fileURLToPath(new URL("./src/components/landing/", import.meta.url))
    for (const file of readdirSync(dir)) {
      expect(readFileSync(dir + file, "utf8").toLowerCase(), file).not.toContain("open source")
    }
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
