// pnpm bench [version]: starts the release bar, lets it settle, then measures idle CPU and
// memory of the bar plus its WebView2 child processes. With a version (pnpm bench 0.1) the
// result is also written to docs/perf/<version>.md. Latency rows arrive with the test client (v0.2).
// BENCH_SECONDS changes the sampling time (default 60).
import { execFileSync, spawn } from "node:child_process"
import { existsSync, writeFileSync } from "node:fs"
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

const bar = spawn(exe, [], { stdio: "ignore", detached: false })
const stop = () => {
  try {
    execFileSync("taskkill", ["/PID", String(bar.pid), "/T", "/F"], { stdio: "ignore" })
  } catch {
    // already gone
  }
}
process.on("exit", stop)

console.log(`Started bar (pid ${bar.pid}), settling ${settleSeconds}s...`)
await sleep(settleSeconds * 1000)

const startedAt = Date.now()
const first = snapshot(bar.pid)
console.log(`Sampling ${seconds}s...`)
await sleep(seconds * 1000)
const last = snapshot(bar.pid)
const elapsed = (Date.now() - startedAt) / 1000
stop()

// CPU as a share of the whole machine, like Task Manager's total.
const cores = os.cpus().length
const cpuPercent = ((last.cpu - first.cpu) / elapsed / cores) * 100
const memoryMb = last.memory / 1024 / 1024
// Working sets of the WebView2 processes share pages (DLLs), so the plain sum counts them
// several times. Private working set is what the processes really own.
const privateMb = last.private / 1024 / 1024

const rows = [
  ["Idle CPU (avg)", `${cpuPercent.toFixed(2)}%`, "< 0.5%", cpuPercent < 0.5],
  [
    "Memory, working set sum (bar + WebView2)",
    `${memoryMb.toFixed(0)} MB`,
    "< 120 MB",
    memoryMb < 120,
  ],
  ["Memory, private only (bar + WebView2)", `${privateMb.toFixed(0)} MB`, "for reference", true],
]
const table = [
  "| What | Result | Budget | |",
  "|---|---|---|---|",
  ...rows.map(
    ([what, result, budget, ok]) => `| ${what} | ${result} | ${budget} | ${ok ? "ok" : "over"} |`,
  ),
  "| Memory growth over 8 h | not measured | < 5 MB | |",
  "| Click to menu / popover, item update, cold start | from v0.2 | see budget file | |",
].join("\n")

console.log(
  `\n${table}\n\n${last.processes} processes, ${cores} logical cores, ${elapsed.toFixed(0)}s sampled.`,
)

if (version) {
  const md = `# Performance ${version}

Measured ${new Date().toISOString().slice(0, 10)} with \`pnpm bench ${version}\` (release build, nothing else started).

${table}

- ${last.processes} processes counted (the bar and its WebView2 children), ${cores} logical cores.
- CPU: total over ${elapsed.toFixed(0)} s after ${settleSeconds} s of settling, divided by cores.
- Machine: ${os.cpus()[0].model.trim()}, ${(os.totalmem() / 1024 ** 3).toFixed(0)} GB RAM.
`
  writeFileSync(`${root}docs/perf/${version}.md`, md)
  console.log(`Wrote docs/perf/${version}.md`)
}
