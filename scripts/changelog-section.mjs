// Prints one version's section of a changelog (used as the GitHub release body).
//   node scripts/changelog-section.mjs CHANGELOG.md v0.1.0
//   node scripts/changelog-section.mjs packages/sdk/CHANGELOG.md sdk-v0.1.0
// Both release workflows call it. With no matching section it prints a one-line title instead,
// so a release never fails just because the notes are missing.
import { readFileSync } from "node:fs"
import { resolve } from "node:path"
import { fileURLToPath } from "node:url"

/** "v0.1.0" and "sdk-v0.1.0" both give "0.1.0". */
export function versionFromTag(tag) {
  return (tag ?? "").replace(/^(sdk-)?v/, "")
}

/** The text under "## [version]" up to the next "## [" heading, or null if there is none. */
export function changelogSection(text, version) {
  const start = text.indexOf(`## [${version}]`)
  if (!version || start === -1) return null
  const rest = text.slice(start)
  const next = rest.indexOf("\n## [", 1)
  // Drop the "## [x.y.z] - date" heading itself: the release already has a title.
  return (next === -1 ? rest : rest.slice(0, next)).split("\n").slice(1).join("\n").trim()
}

// Only run when called from the command line (not when a test imports the functions above).
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [path, tag] = process.argv.slice(2)
  if (!path || !tag) {
    console.error("Usage: node scripts/changelog-section.mjs <CHANGELOG path> <tag>")
    process.exit(2)
  }
  const section = changelogSection(readFileSync(path, "utf8"), versionFromTag(tag))
  console.log(section ?? `Barista ${tag}`)
}
