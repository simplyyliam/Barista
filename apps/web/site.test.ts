// Guards the website setup: static export stays on, every docs page is real and listed, and links are public.
import { readdirSync, readFileSync } from "node:fs"
import { fileURLToPath } from "node:url"
import { describe, expect, it } from "vitest"

const read = (path: string) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), "utf8")

describe("website", () => {
  it("is a static export", () => {
    expect(read("./next.config.mjs")).toMatch(/output:\s*"export"/)
  })

  it("has no pricing or payment copy (Barista is free)", () => {
    const dir = fileURLToPath(new URL("./src/components/landing/", import.meta.url))
    for (const file of readdirSync(dir).filter((f) => f.endsWith(".tsx") || f.endsWith(".ts"))) {
      expect(readFileSync(dir + file, "utf8"), file).not.toMatch(/pricing|\$5|one-off|upgrade/i)
    }
  })

  it("lists every docs page in meta.json", () => {
    const meta = JSON.parse(read("./content/docs/meta.json")) as { pages: string[] }
    const listed = meta.pages.filter((page) => !page.startsWith("---"))
    const files = readdirSync(fileURLToPath(new URL("./content/docs/", import.meta.url)))
      .filter((file) => file.endsWith(".mdx"))
      .map((file) => file.replace(/\.mdx$/, ""))
    expect([...listed].sort()).toEqual([...files].sort())
  })

  it("has no placeholder docs page left", () => {
    const dir = fileURLToPath(new URL("./content/docs/", import.meta.url))
    for (const file of readdirSync(dir).filter((f) => f.endsWith(".mdx"))) {
      expect(readFileSync(dir + file, "utf8"), file).not.toMatch(/Coming in v0\./)
    }
  })

  it("points the download at the public releases repo", () => {
    expect(read("./src/components/landing/site.ts")).toContain("simplyyliam/barista-releases/releases/latest")
  })
})
