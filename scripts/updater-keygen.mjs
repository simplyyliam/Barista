// One-time setup for signed auto-updates: pnpm updater:keygen
// 1. Creates the signing key in .private/updater/ (git-ignored). Tauri asks you for a password:
//    keep it in your password manager, not in a file.
// 2. Copies the PUBLIC key into tauri.conf.json, so installed apps can check that an update
//    really came from you.
// 3. Prints the two GitHub secrets the release Action needs.
// It never overwrites an existing key: losing it means installs can't update.
import { execFileSync } from "node:child_process"
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const keyPath = join(root, ".private", "updater", "barista-updater.key")
const confPath = join(root, "apps", "bar", "src-tauri", "tauri.conf.json")

if (existsSync(keyPath)) {
  console.error(`A key already exists at ${keyPath}. Not overwriting it.`)
  process.exit(1)
}
mkdirSync(dirname(keyPath), { recursive: true })

// Interactive: Tauri prompts for the password in this terminal.
// On Windows pnpm is a .cmd file and needs a shell, so the path is quoted (spaces in the path).
const keyArg = process.platform === "win32" ? `"${keyPath}"` : keyPath
execFileSync("pnpm", ["--filter", "@barista/bar", "tauri", "signer", "generate", "-w", keyArg], {
  stdio: "inherit",
  shell: process.platform === "win32",
})

// Only the .pub file (the public half) is read here, never the private key.
const conf = JSON.parse(readFileSync(confPath, "utf8"))
conf.plugins ??= {}
conf.plugins.updater ??= {}
conf.plugins.updater.pubkey = readFileSync(`${keyPath}.pub`, "utf8").trim()
// Makes `tauri build` produce the signed update files that latest.json points at.
conf.bundle ??= {}
conf.bundle.createUpdaterArtifacts = true
writeFileSync(confPath, `${JSON.stringify(conf, null, 2)}\n`)

console.log(`
Done. The public key is now in apps/bar/src-tauri/tauri.conf.json (commit that).
The updater plugin and its endpoints are wired in v0.9; until then this only prepares signing.

Add these GitHub secrets so the release Action can sign updates:
  gh secret set TAURI_SIGNING_PRIVATE_KEY < "${keyPath}"
  gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD

Back up ${keyPath} somewhere safe (outside this computer).
`)
