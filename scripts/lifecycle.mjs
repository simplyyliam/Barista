// pnpm lifecycle: the v0.5 "Never stuck" checks against a real release bar.
//   1. kill the test client 20 times: its item must be gone each time (and none left at the end)
//   2. a client that never answers pings loses its items in 15 to 20 s
//   3. kill the bar and start it again, 5 times with 3 clients: every item is back within 2 s
//   4. start a client first, then the bar: the item appears
//   5. quit the bar the polite way: a client hears `shutdown { quit }` before the pipe closes
// It starts (and stops) its own bar, so no other bar may be running: it would own the pipe.
// Needs a release build (`pnpm --filter @barista/bar tauri build --no-bundle`).
import { execFileSync, execSync, spawn } from "node:child_process"
import { existsSync } from "node:fs"
import net from "node:net"
import { fileURLToPath } from "node:url"

const root = fileURLToPath(new URL("..", import.meta.url))
const exe = `${root}target/release/barista.exe`
const clientScript = `${root}scripts/lifecycle-client.mjs`
const PIPE = "\\\\.\\pipe\\barista"
const only = process.argv.slice(2).filter((a) => !a.startsWith("-"))
const run = (n) => only.length === 0 || only.includes(String(n))

if (!existsSync(exe)) {
  console.error("No release build. Run: pnpm --filter @barista/bar tauri build --no-bundle")
  process.exit(1)
}
const running = execFileSync("tasklist", ["/FI", "IMAGENAME eq barista.exe", "/NH"], {
  encoding: "utf8",
})
if (running.toLowerCase().includes("barista.exe")) {
  console.error("A bar is already running. Close it first: this script starts its own.")
  process.exit(1)
}
execSync("pnpm --filter @barista/sdk build", { stdio: "ignore" })

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const stats = (list) => {
  const sorted = [...list].sort((a, b) => a - b)
  return { median: sorted[Math.floor(sorted.length / 2)], worst: sorted.at(-1) }
}
const fmt = (n) => `${Math.round(n)} ms`

// ---- raw protocol, so the script depends on nothing but the wire format ----
function frame(message) {
  const json = Buffer.from(JSON.stringify(message))
  const out = Buffer.alloc(4 + json.length)
  out.writeUInt32LE(json.length)
  json.copy(out, 4)
  return out
}

/** Opens a connection and says hello. `onMessage` sees everything the bar sends. */
function open({ version, appName, onMessage = () => {}, pong = false }) {
  return new Promise((resolve, reject) => {
    const socket = net.connect(PIPE)
    let buffer = Buffer.alloc(0)
    const client = {
      socket,
      send: (m) => socket.write(frame(m)),
      closed: false,
    }
    socket.on("connect", () =>
      client.send({
        type: "hello",
        protocolVersion: version,
        appName,
        pid: process.pid,
        sdk: { name: "lifecycle", version: "0" },
      }),
    )
    socket.on("data", (chunk) => {
      buffer = Buffer.concat([buffer, chunk])
      while (buffer.length >= 4 && buffer.length >= 4 + buffer.readUInt32LE(0)) {
        const length = buffer.readUInt32LE(0)
        const message = JSON.parse(buffer.subarray(4, 4 + length).toString())
        buffer = buffer.subarray(4 + length)
        if (message.type === "welcome") resolve(client)
        if (message.type === "ping" && pong) client.send({ type: "pong", seq: message.seq })
        onMessage(message, client)
      }
    })
    socket.on("error", reject)
    socket.on("close", () => (client.closed = true))
  })
}

/**
 * Every item in the bar, or null when the bar does not answer. One connection is kept and asked
 * again and again: reconnecting for every poll would make the pipe busy for the clients we are
 * timing.
 */
let watcher
let latest
async function listItems() {
  try {
    if (!watcher || watcher.closed) {
      watcher = await open({
        version: "0.4", // no heartbeat for the watcher
        appName: "lifecycle-watch",
        onMessage: (m) => {
          if (m.type === "debug.items") latest = m.items
        },
      })
    }
    latest = undefined
    watcher.send({ type: "debug.listItems" })
    const until = Date.now() + 2000
    while (!latest && !watcher.closed && Date.now() < until) await sleep(2)
    return latest ?? null
  } catch {
    watcher = undefined
    return null
  }
}

async function waitFor(check, timeoutMs, stepMs = 20) {
  const start = performance.now()
  while (performance.now() - start < timeoutMs) {
    if (await check()) return performance.now() - start
    await sleep(stepMs)
  }
  return null
}

const has = (items, app) => items?.some((i) => i.appName === app) ?? false
const count = (items, app) => items?.filter((i) => i.appName === app).length ?? 0

// ---- processes ----
let bar
const clients = new Set()

function startBar() {
  // The debug port lets step 5 call `quit_bar` like the UI would.
  bar = spawn(exe, [], {
    stdio: "ignore",
    env: {
      ...process.env,
      BARISTA_DEBUG: "1",
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: "--remote-debugging-port=9222",
    },
  })
  return bar
}

/** Quits the bar the way the UI would: the `quit_bar` command, called in the bar's main page. */
async function quitBarPolitely() {
  const pages = await (await fetch("http://127.0.0.1:9222/json")).json()
  const page = pages.find((p) => p.type === "page" && !/panel|menu|debug/.test(p.url + p.title))
  if (!page) throw new Error("no page on the debug port")
  const ws = new WebSocket(page.webSocketDebuggerUrl)
  await new Promise((resolve, reject) => {
    ws.onopen = resolve
    ws.onerror = reject
  })
  ws.send(
    JSON.stringify({
      id: 1,
      method: "Runtime.evaluate",
      params: { expression: "window.__TAURI_INTERNALS__.invoke('quit_bar')", awaitPromise: false },
    }),
  )
  await sleep(200)
  ws.close()
}

function killBar() {
  try {
    execFileSync("taskkill", ["/PID", String(bar.pid), "/F", "/T"], { stdio: "ignore" })
  } catch {
    // already gone
  }
}

function startClient() {
  const child = spawn(process.execPath, [clientScript], { stdio: "ignore", windowsHide: true })
  clients.add(child)
  child.on("exit", () => clients.delete(child))
  return child
}

function stopAll() {
  for (const c of clients) c.kill("SIGKILL")
  if (bar) killBar()
}
process.on("exit", stopAll)
process.on("SIGINT", () => process.exit(1))

async function barReady(timeoutMs = 30_000) {
  return waitFor(async () => (await listItems()) !== null, timeoutMs, 50)
}

const rows = []
const row = (what, result, budget, ok) => rows.push([what, result, budget, ok ? "ok" : "FAIL"])

console.log("Starting the bar...")
startBar()
if ((await barReady()) === null) {
  console.error("The bar did not come up in 30 s")
  process.exit(1)
}
await sleep(3000) // let the window and the services settle

// 1 ---------------------------------------------------------------------------------------
if (run(1)) {
  console.log("1. Killing the test client 20 times...")
  const times = []
  let stuck = 0
  for (let n = 0; n < 20; n++) {
    const client = startClient()
    const appeared = await waitFor(async () => has(await listItems(), "Focus"), 15_000)
    if (appeared === null) {
      console.error(`   round ${n + 1}: the item never appeared`)
      stuck++
      client.kill("SIGKILL")
      continue
    }
    const killedAt = performance.now()
    client.kill("SIGKILL")
    const gone = await waitFor(async () => !has(await listItems(), "Focus"), 5000, 10)
    if (gone === null) stuck++
    else times.push(performance.now() - killedAt)
  }
  const left = count(await listItems(), "Focus")
  const s = stats(times.length ? times : [0])
  row(
    "Kill the client: its item is gone (median / worst of 20)",
    `${fmt(s.median)} / ${fmt(s.worst)}, ${stuck} dead icons, ${left} left at the end`,
    "0 dead icons",
    stuck === 0 && left === 0,
  )
}

// 2 ---------------------------------------------------------------------------------------
if (run(2)) {
  console.log("2. A client that never answers pings (takes up to 20 s)...")
  const client = await open({ version: "0.5", appName: "Frozen", pong: false })
  client.send({ type: "item.create", id: "stuck", label: "frozen" })
  await waitFor(async () => has(await listItems(), "Frozen"), 3000)
  const start = performance.now()
  const gone = await waitFor(async () => !has(await listItems(), "Frozen"), 30_000, 100)
  client.socket.destroy()
  row(
    "Client that never pongs: its item is removed",
    gone === null ? "never removed" : `${((performance.now() - start) / 1000).toFixed(1)} s`,
    "15 to 20 s",
    gone !== null && gone >= 14_000 && gone <= 21_000,
  )
}

// 3 ---------------------------------------------------------------------------------------
if (run(3)) {
  console.log("3. Killing the bar and starting it again, 5 times, with 3 clients...")
  const kids = [startClient(), startClient(), startClient()]
  await waitFor(async () => count(await listItems(), "Focus") === 3, 15_000)
  const times = []
  const fromLaunch = []
  let failed = 0
  for (let n = 0; n < 5; n++) {
    killBar()
    await sleep(500)
    const launchedAt = performance.now()
    startBar()
    // The clock starts when the bar answers on its pipe: its own startup is not the SDK's doing.
    const pipeUp = await waitFor(async () => (await listItems()) !== null, 30_000, 5)
    if (pipeUp === null) {
      failed++
      continue
    }
    const t0 = performance.now()
    const back = await waitFor(async () => count(await listItems(), "Focus") === 3, 5000, 10)
    if (back === null) failed++
    else {
      times.push(performance.now() - t0)
      fromLaunch.push(performance.now() - launchedAt)
    }
  }
  const s = stats(times.length ? times : [0])
  const l = stats(fromLaunch.length ? fromLaunch : [0])
  row(
    "Restart the bar: all 3 clients' items are back (median / worst of 5), counted from the pipe answering",
    `${fmt(s.median)} / ${fmt(s.worst)}, ${failed} failed`,
    "< 2000 ms",
    failed === 0 && s.worst < 2000,
  )
  row(
    "Same restart, counted from launching the bar.exe (its own startup included)",
    `${fmt(l.median)} / ${fmt(l.worst)}`,
    "info",
    true,
  )
  kids.forEach((k) => k.kill("SIGKILL"))
  await waitFor(async () => !has(await listItems(), "Focus"), 5000)
}

// 4 ---------------------------------------------------------------------------------------
if (run(4)) {
  console.log("4. Client first, bar second...")
  killBar()
  await sleep(500)
  const client = startClient()
  await sleep(2500) // the client is up and waiting, with no bar
  startBar()
  const pipeUp = await barReady()
  const t0 = performance.now()
  const shown = pipeUp === null ? null : await waitFor(async () => has(await listItems(), "Focus"), 5000, 10)
  row(
    "Client started before the bar: its item appears",
    shown === null ? "never appeared" : fmt(performance.now() - t0),
    "< 2000 ms",
    shown !== null && performance.now() - t0 < 2000,
  )
  client.kill("SIGKILL")
  await waitFor(async () => !has(await listItems(), "Focus"), 5000)
}

// 5 ---------------------------------------------------------------------------------------
if (run(5)) {
  console.log("5. Quitting the bar the polite way...")
  const heard = []
  const listener = await open({
    version: "0.5",
    appName: "Listener",
    pong: true,
    onMessage: (m) => heard.push(m.type === "shutdown" ? `shutdown:${m.reason}` : m.type),
  })
  listener.send({ type: "item.create", id: "listening", label: "hi" })
  await sleep(300)
  try {
    await quitBarPolitely()
  } catch (error) {
    console.error(`   could not call quit_bar: ${error.message}`)
  }
  await waitFor(() => listener.closed, 5000, 20)
  row(
    "Bar quits politely: the client hears shutdown { quit } before the pipe closes",
    heard.includes("shutdown:quit") ? "heard it" : `heard: ${heard.join(", ")}`,
    "shutdown:quit",
    heard.includes("shutdown:quit") && listener.closed,
  )
}

console.log("\n| What | Result | Budget | |\n|---|---|---|---|")
for (const r of rows) console.log(`| ${r.join(" | ")} |`)
process.exit(rows.some((r) => r[3] !== "ok") ? 1 : 0)
