// pnpm bench [version]: starts the release bar, lets it settle, then measures idle CPU and
// memory of the bar plus its WebView2 child processes. With a version (pnpm bench 0.1) the
// result is also written to docs/perf/<version>.md. Since v0.2 it also measures item latency
// over the pipe and the bar's CPU while the test client ticks a label every second.
// BENCH_SECONDS changes the sampling time of each CPU measurement (default 60).
import { execFileSync, spawn } from "node:child_process"
import { existsSync, writeFileSync } from "node:fs"
import net from "node:net"
import os from "node:os"
import { fileURLToPath } from "node:url"

const root = fileURLToPath(new URL("..", import.meta.url))
const exe = `${root}target/release/barista.exe`
const version = process.argv[2]
const seconds = Number(process.env.BENCH_SECONDS ?? 60)
const settleSeconds = 10

if (!existsSync(exe)) {
  console.error("No release build. Run: pnpm --filter @barista/bar tauri build --no-bundle")
  process.exit(1)
}

// A second bar would share the first one's WebView2 process, so the numbers would be wrong.
const running = execFileSync("tasklist", ["/FI", "IMAGENAME eq barista.exe", "/NH"], {
  encoding: "utf8",
})
if (running.toLowerCase().includes("barista.exe")) {
  console.error(
    "barista.exe is already running. Close the bar (and pnpm dev) first, then run pnpm bench again.",
  )
  process.exit(1)
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))

// One PowerShell call returns CPU seconds and working set for the bar and all its descendants.
function snapshot(barPid) {
  const script = `
    $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId
    $ids = [System.Collections.Generic.HashSet[int]]::new(); [void]$ids.Add(${barPid})
    do { $n = $ids.Count
      foreach ($p in $all) { if ($ids.Contains([int]$p.ParentProcessId)) { [void]$ids.Add([int]$p.ProcessId) } }
    } while ($ids.Count -ne $n)
    $procs = Get-Process -Id @($ids) -ErrorAction SilentlyContinue
    $private = (Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object { $ids.Contains([int]$_.IDProcess) } | Measure-Object -Property WorkingSetPrivate -Sum).Sum
    [pscustomobject]@{
      private = $private
      cpu = ($procs | Measure-Object -Property CPU -Sum).Sum
      memory = ($procs | Measure-Object -Property WorkingSet64 -Sum).Sum
      processes = @($procs).Count
    } | ConvertTo-Json -Compress`
  const out = execFileSync("powershell", ["-NoProfile", "-Command", script], { encoding: "utf8" })
  return JSON.parse(out)
}

// BARISTA_DEBUG turns on the debug.* messages in a release bar, which the latency probe needs.
const bar = spawn(exe, [], {
  stdio: "ignore",
  detached: false,
  env: { ...process.env, BARISTA_DEBUG: "1" },
})
const stop = () => {
  // Ask nicely first (WM_CLOSE) so the bar gives its screen space back, then force.
  for (const flags of [[], ["/F", "/T"]]) {
    try {
      execFileSync("taskkill", ["/PID", String(bar.pid), ...flags], { stdio: "ignore" })
    } catch {
      return // already gone
    }
    if (flags.length === 0) {
      const until = Date.now() + 3000
      while (Date.now() < until && bar.exitCode === null) {
        execFileSync("powershell", ["-NoProfile", "-Command", "Start-Sleep -Milliseconds 200"])
      }
      if (bar.exitCode !== null) return
    }
  }
}
process.on("exit", stop)

console.log(`Started bar (pid ${bar.pid}), settling ${settleSeconds}s...`)
await sleep(settleSeconds * 1000)

/** CPU of the bar for `seconds`, as a share of the whole machine (like Task Manager's total). */
const cores = os.cpus().length
async function sampleCpu() {
  const startedAt = Date.now()
  const first = snapshot(bar.pid)
  await sleep(seconds * 1000)
  const last = snapshot(bar.pid)
  const elapsed = (Date.now() - startedAt) / 1000
  return { last, elapsed, percent: ((last.cpu - first.cpu) / elapsed / cores) * 100 }
}

// A minimal pipe client (no SDK), so the numbers do not include SDK overhead.
function pipeClient() {
  const socket = net.connect(String.raw`\\.\pipe\barista`)
  let buffer = Buffer.alloc(0)
  const waiting = []
  socket.on("data", (chunk) => {
    buffer = Buffer.concat([buffer, chunk])
    while (buffer.length >= 4 && buffer.length >= 4 + buffer.readUInt32LE(0)) {
      const length = buffer.readUInt32LE(0)
      const message = JSON.parse(buffer.subarray(4, 4 + length).toString())
      buffer = buffer.subarray(4 + length)
      waiting.shift()?.(message)
    }
  })
  const write = (message) => {
    const json = Buffer.from(JSON.stringify(message))
    const out = Buffer.alloc(4 + json.length)
    out.writeUInt32LE(json.length)
    json.copy(out, 4)
    socket.write(out)
  }
  return {
    ready: new Promise((resolve, reject) => {
      socket.on("error", reject)
      socket.on("connect", resolve)
    }),
    /** Sends a message and resolves with the next reply. */
    ask(message) {
      return new Promise((resolve) => {
        waiting.push(resolve)
        write(message)
      })
    },
    write,
    close: () => socket.destroy(),
  }
}

/** Sends `change`, then asks for the item list until `done(items)` is true. Returns ms. */
async function timeUntil(client, change, done) {
  const t0 = performance.now()
  client.write(change)
  for (;;) {
    const reply = await client.ask({ type: "debug.listItems" })
    if (done(reply.items)) return performance.now() - t0
  }
}

const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)]

async function measureLatency() {
  const client = pipeClient()
  const t0 = performance.now()
  await client.ready
  const welcome = await client.ask({
    type: "hello",
    protocolVersion: "0.2",
    appName: "bench",
    pid: process.pid,
    sdk: { name: "bench", version: "0" },
  })
  if (welcome.type !== "welcome") throw new Error(`bench hello failed: ${JSON.stringify(welcome)}`)
  const connectMs = performance.now() - t0
  const mine = (items) => items.filter((i) => i.appName === "bench")
  const create = [],
    update = []
  for (let n = 0; n < 30; n++) {
    create.push(
      await timeUntil(
        client,
        { type: "item.create", id: "probe", label: "0" },
        (i) => mine(i).length === 1,
      ),
    )
    update.push(
      await timeUntil(client, { type: "item.update", id: "probe", label: String(n + 1) }, (i) =>
        mine(i).some((x) => x.label === String(n + 1)),
      ),
    )
    client.write({ type: "item.remove", id: "probe" })
  }
  client.close()
  return { connectMs, create, update }
}

console.log(`Sampling idle CPU ${seconds}s...`)
const idle = await sampleCpu()

console.log("Measuring item latency (30 rounds)...")
const latency = await measureLatency()

console.log(
  `Starting the test client, sampling CPU ${seconds}s with a label ticking every second...`,
)
const client = spawn(process.execPath, [`${root}examples/test-client/src/index.ts`], {
  stdio: "ignore",
})
await sleep(2000) // let it connect before sampling
const ticking = await sampleCpu()
client.kill()
stop()

const last = idle.last
const elapsed = idle.elapsed
const cpuPercent = idle.percent
const memoryMb = last.memory / 1024 / 1024
// The budget counts private working set (what the processes really own). The plain working
// set sum counts pages the WebView2 processes share (DLLs) several times: reference only.
const privateMb = last.private / 1024 / 1024

const rows = [
  ["Idle CPU (avg)", `${cpuPercent.toFixed(2)}%`, "< 0.5%", cpuPercent < 0.5],
  [
    "CPU with one item updating every second",
    `${ticking.percent.toFixed(2)}%`,
    "< 0.5%",
    ticking.percent < 0.5,
  ],
  [
    "Connect + hello to welcome",
    `${latency.connectMs.toFixed(1)} ms`,
    "< 100 ms",
    latency.connectMs < 100,
  ],
  [
    "item.create until it is in the registry (median / worst of 30)",
    `${median(latency.create).toFixed(1)} / ${Math.max(...latency.create).toFixed(1)} ms`,
    "< 100 ms",
    Math.max(...latency.create) < 100,
  ],
  [
    "item.update (label) until it is in the registry (median / worst of 30)",
    `${median(latency.update).toFixed(1)} / ${Math.max(...latency.update).toFixed(1)} ms`,
    "< 100 ms",
    Math.max(...latency.update) < 100,
  ],
  ["Memory, private (bar + WebView2)", `${privateMb.toFixed(0)} MB`, "< 120 MB", privateMb < 120],
  ["Memory, working set sum (reference)", `${memoryMb.toFixed(0)} MB`, "counts shared pages", true],
]
const table = [
  "| What | Result | Budget | |",
  "|---|---|---|---|",
  ...rows.map(
    ([what, result, budget, ok]) => `| ${what} | ${result} | ${budget} | ${ok ? "ok" : "over"} |`,
  ),
  "| Memory growth over 8 h | not measured | < 5 MB | |",
  "| Click to menu / popover, cold start | from v0.3 | see budget file | |",
].join("\n")

console.log(
  `\n${table}\n\n${last.processes} processes, ${cores} logical cores, ${elapsed.toFixed(0)}s sampled.`,
)

if (version) {
  const md = `# Performance ${version}

Measured ${new Date().toISOString().slice(0, 10)} with \`pnpm bench ${version}\` (release build, nothing else started).

${table}

- ${last.processes} processes counted (the bar and its WebView2 children), ${cores} logical cores.
- CPU: total over ${elapsed.toFixed(0)} s after ${settleSeconds} s of settling, divided by cores. The ticking row is the same with \`examples/test-client\` running (one label update per second).
- Latency is measured over the pipe with a minimal client: the time from sending the message until \`debug.listItems\` shows it. The UI sees it about one frame (16 ms) later, because \`items-changed\` is coalesced; the render time is the UI's.
- Machine: ${os.cpus()[0].model.trim()}, ${(os.totalmem() / 1024 ** 3).toFixed(0)} GB RAM.
`
  writeFileSync(`${root}docs/perf/${version}.md`, md)
  console.log(`Wrote docs/perf/${version}.md`)
}
