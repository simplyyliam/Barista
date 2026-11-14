// Fails when a library in any package.json or Cargo.toml has no row in CREDITS.md.
import { readFileSync, readdirSync, existsSync } from "node:fs"
import { join } from "node:path"
import { fileURLToPath } from "node:url"
import { describe, expect, it } from "vitest"

const root = fileURLToPath(new URL("..", import.meta.url))

/** Every folder that holds a package.json or Cargo.toml (the workspace globs, one level deep). */
function projectDirs(): string[] {
  const dirs = [root]
  for (const group of ["apps", "packages", "examples", "crates"]) {
    if (!existsSync(join(root, group))) continue
    for (const entry of readdirSync(join(root, group), { withFileTypes: true })) {
      if (entry.isDirectory()) dirs.push(join(root, group, entry.name))
    }
  }
  dirs.push(join(root, "apps", "bar", "src-tauri"))
  return dirs
}

function npmDependencies(dir: string): string[] {
  const file = join(dir, "package.json")
  if (!existsSync(file)) return []
  const pkg = JSON.parse(readFileSync(file, "utf8"))
  const all = { ...pkg.dependencies, ...pkg.devDependencies }
  // Our own workspace packages (@barista/* private ones, @baristabar/* the published ones) don't need a credit.
  return Object.keys(all).filter(
    (name) => !name.startsWith("@barista/") && !name.startsWith("@baristabar/"),
  )
}

function cargoDependencies(dir: string): string[] {
  const file = join(dir, "Cargo.toml")
  if (!existsSync(file)) return []
  const names: string[] = []
  let inDeps = false
  for (const line of readFileSync(file, "utf8").split(/\r?\n/)) {
    const section = line.match(/^\[(.+)\]\s*$/)
    if (section) inDeps = /(^|\.)(dev-|build-)?dependencies$/.test(section[1] ?? "")
    else if (inDeps) {
      const dep = line.match(/^([A-Za-z0-9_-]+)\s*=/)
      // barista-protocol is our own crate (a path dependency), so it needs no credit.
      if (dep?.[1] && dep[1] !== "barista-protocol") names.push(dep[1])
    }
  }
  return names
}

describe("CREDITS.md", () => {
  const credits = readFileSync(join(root, "CREDITS.md"), "utf8")
  const credited = new Set(
    [...credits.matchAll(/^\|\s*([^|\s][^|]*?)\s*\|/gm)].map((m) => m[1] as string),
  )

  it("has a row for every dependency", () => {
    const used = new Set(
      projectDirs().flatMap((d) => [...npmDependencies(d), ...cargoDependencies(d)]),
    )
    const missing = [...used].filter((name) => !credited.has(name))
    expect(missing).toEqual([])
  })
})
