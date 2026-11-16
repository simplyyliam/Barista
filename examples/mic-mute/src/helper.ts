// Starts win-helper (a tiny Rust program, ADR 0030) and talks to it in JSON lines.
// Node cannot reach the microphone's mute switch, so the helper does that one thing.
import { spawn } from "node:child_process"
import { existsSync } from "node:fs"
import { createInterface } from "node:readline"
import { fileURLToPath } from "node:url"

/** Where the helper is: `BARISTA_WIN_HELPER`, else the cargo build next to this repo. */
function helperPath(): string {
  const fromEnv = process.env["BARISTA_WIN_HELPER"]
  if (fromEnv) return fromEnv
  for (const profile of ["release", "debug"]) {
    const path = fileURLToPath(new URL(`../../../target/${profile}/win-helper.exe`, import.meta.url))
    if (existsSync(path)) return path
  }
  throw new Error("win-helper.exe not found: run `cargo build -p barista-win-helper` (or set BARISTA_WIN_HELPER)")
}

export interface Helper {
  send(command: Record<string, unknown>): void
  close(): void
}

/** Runs the helper in `mode`. `onEvent` gets each line it prints; `onExit` runs if it stops. */
export function startHelper(mode: "--mic" | "--media", onEvent: (event: unknown) => void, onExit: () => void): Helper {
  const child = spawn(helperPath(), [mode], { stdio: ["pipe", "pipe", "inherit"], windowsHide: true })
  createInterface({ input: child.stdout }).on("line", (line) => {
    try {
      onEvent(JSON.parse(line))
    } catch {
      // Not JSON: ignore it.
    }
  })
  child.on("exit", onExit)
  // A broken pipe on a dying helper must not crash the app.
  child.stdin.on("error", () => {})
  return {
    send: (command) => void child.stdin.write(`${JSON.stringify(command)}\n`),
    close: () => child.kill(),
  }
}
