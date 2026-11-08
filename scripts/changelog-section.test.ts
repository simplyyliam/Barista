import { execFileSync } from "node:child_process"
import { describe, expect, it } from "vitest"
// @ts-expect-error plain .mjs script without types
import { changelogSection, versionFromTag } from "./changelog-section.mjs"

const CHANGELOG = `# Changelog

## [Unreleased]

## [0.2.0] - 2026-02-01

### Added

- Menus

## [0.1.0] - 2026-01-01

- First release
`

describe("changelog-section", () => {
  it("strips v and sdk-v from tags", () => {
    expect(versionFromTag("v0.1.0")).toBe("0.1.0")
    expect(versionFromTag("sdk-v1.2.3")).toBe("1.2.3")
  })

  it("returns only the requested version, without its heading", () => {
    expect(changelogSection(CHANGELOG, "0.2.0")).toBe("### Added\n\n- Menus")
    expect(changelogSection(CHANGELOG, "0.1.0")).toBe("- First release")
  })

  it("returns null for an unknown version", () => {
    expect(changelogSection(CHANGELOG, "9.9.9")).toBeNull()
  })

  it("works from the command line against the real CHANGELOG", () => {
    const out = execFileSync("node", ["scripts/changelog-section.mjs", "CHANGELOG.md", "v9.9.9"], {
      encoding: "utf8",
    })
    expect(out.trim()).toBe("Barista v9.9.9")
  })
})
