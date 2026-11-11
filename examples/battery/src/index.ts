// A battery in the menu bar: a wide icon with a fill that follows the charge, and the percentage.
// The icon (icon.ts) is custom-drawn: the fill level and the charging / plugged marks change.
import { execFile } from "node:child_process"
import * as BA from "@barista/sdk"
import { batteryIcon } from "./icon.ts"

interface BatteryStatus {
  hasBattery: boolean
  percentage?: number
  charging?: boolean
  pluggedIn?: boolean
}

// GetSystemPowerStatus is what the Windows battery indicator uses, so the number matches it (with
// a charge limit at 80 %, it says 80; the WMI `EstimatedChargeRemaining` said 79).
const POWER_SHELL_SCRIPT = `
$ErrorActionPreference = 'Stop'
Add-Type @"
using System.Runtime.InteropServices;
public class Pwr {
  [StructLayout(LayoutKind.Sequential)]
  public struct S { public byte Ac; public byte Flag; public byte Pct; public byte Saver; public int Life; public int Full; }
  [DllImport("kernel32.dll")] public static extern bool GetSystemPowerStatus(out Pwr.S s);
}
"@
$s = New-Object Pwr+S
if (-not [Pwr]::GetSystemPowerStatus([ref]$s)) { throw 'GetSystemPowerStatus failed' }
# Flag: 8 = charging, 128 = no battery. Pct: 255 = unknown. Ac: 1 = plugged in.
$has = (($s.Flag -band 128) -eq 0) -and ($s.Pct -ne 255)
@{
  hasBattery = $has
  percentage = if ($has) { [int]$s.Pct } else { -1 }
  charging = (($s.Flag -band 8) -ne 0)
  pluggedIn = ($s.Ac -eq 1)
} | ConvertTo-Json -Compress
`

function readBattery(): Promise<BatteryStatus> {
  return new Promise((resolve, reject) => {
    execFile(
      "powershell.exe",
      ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", POWER_SHELL_SCRIPT],
      { encoding: "utf8", windowsHide: true, timeout: 8000 },
      (error, stdout) => {
        if (error) {
          reject(error)
          return
        }

        try {
          resolve(JSON.parse(stdout.trim()) as BatteryStatus)
        } catch (parseError) {
          reject(new Error(`Could not parse Windows battery status: ${String(parseError)}`))
        }
      },
    )
  })
}

const bar = await BA.connect({ appName: "Battery" })
const item = await BA.StatusItem.create({
  id: "battery",
  icon: batteryIcon(0, false, false),
  label: "Battery",
  tooltip: "Reading battery status",
})

let stopping = false
let refreshing = false

async function refresh() {
  if (stopping || refreshing) return
  refreshing = true

  try {
    const status = await readBattery()

    if (!status.hasBattery || typeof status.percentage !== "number" || status.percentage < 0) {
      item.setLabel("N/A")
      item.setTooltip("Battery status is unavailable on this device")
      return
    }

    const percentage = Math.min(100, Math.max(0, Math.round(status.percentage)))
    const charging = status.charging === true
    const powerState = charging ? "Charging" : status.pluggedIn ? "Plugged in" : "On battery"

    item.setIcon(batteryIcon(percentage, charging, status.pluggedIn === true))
    item.setLabel(`${percentage}%`)
    item.setTooltip(`${percentage}% · ${powerState}`)
  } catch (error) {
    console.error("Could not retrieve battery status:", error)
    item.setTooltip("Could not read battery status")
  } finally {
    refreshing = false
  }
}

function shutdown() {
  if (stopping) return
  stopping = true
  clearInterval(timer)
  item.remove()
  bar.close()
}

const timer = setInterval(() => void refresh(), 30_000)

item.on("click", () => void refresh())
item.on("error", (error) => {
  console.error(`Barista refused the battery item: ${error.code}: ${error.message}`)
})

bar.on("close", () => {
  clearInterval(timer)
  process.exit(0)
})

process.on("SIGINT", shutdown)
process.on("SIGTERM", shutdown)

void refresh()
