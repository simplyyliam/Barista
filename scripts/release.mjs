// pnpm release: pushes the newest release tag that /release tag prepared.
// It pushes ONLY that tag (main is protected: it only changes through merged PRs), after
// checking main and the tag again, showing what goes out, and asking first.
// Pushing the tag starts the GitHub Action: v* builds installers and the update (draft release),
// sdk-v* publishes the SDK to npm.
import { execFileSync } from "node:child_process"
import { createInterface } from "node:readline/promises"

const git = (...args) => execFileSync("git", args, { encoding: "utf8" }).trim()
const fail = (message) => {
  console.error(`\n✗ ${message}\n`)
  process.exit(1)
}

// Newest tag of either track (app: v0.1.0, SDK: sdk-v0.1.0), by creation date.
const tag = git("tag", "--sort=-creatordate", "--list", "v*", "sdk-v*").split("\n")[0]
if (!tag) fail("There is no release tag. Run /release tag first.")

if (git("branch", "--show-current") !== "main") fail("Switch to main first.")
if (git("status", "--porcelain")) fail("The working tree isn't clean. Commit or stash first.")
if (git("ls-files", ".private"))
  fail(".private/ has tracked files. The signing key must never be pushed.")
if (git("rev-list", "-n", "1", tag) !== git("rev-parse", "HEAD")) {
  fail(`${tag} doesn't point at HEAD. Pull main, then run /release tag again.`)
}
git("fetch", "origin", "--tags", "--quiet")
if (git("ls-remote", "--tags", "origin", tag)) fail(`${tag} is already on GitHub.`)

console.log(`\nTag ${tag}\n${git("tag", "-l", "--format=%(contents)", tag)}\n`)

const rl = createInterface({ input: process.stdin, output: process.stdout })
const answer = await rl.question(`Push ${tag} to origin? [y/N] `)
rl.close()
if (answer.trim().toLowerCase() !== "y") fail("Nothing was pushed.")

execFileSync("git", ["push", "origin", tag], { stdio: "inherit" })
console.log(`\n✓ Pushed ${tag}. Watch the release Action on GitHub (Actions tab).`)
if (tag.startsWith("sdk-v")) {
  console.log("It publishes the SDK to npm and creates the GitHub release.")
} else {
  console.log("When it's done, test the installers, then publish the draft on the Releases page.")
  console.log("Publishing is what makes installed copies update.")
}
