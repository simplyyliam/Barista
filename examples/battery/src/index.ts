import { execFile } from "node:child_process"
import * as BA from "@barista/sdk"

interface BatteryStatus {
  hasBattery: boolean
  percentage?: number
  charging?: boolean
  pluggedIn?: boolean
}

const POWER_SHELL_SCRIPT = `
$ErrorActionPreference = 'Stop'
$battery = Get-CimInstance -ClassName Win32_Battery | Select-Object -First 1

if ($null -eq $battery) {
  @{ hasBattery = $false } | ConvertTo-Json -Compress
} else {
  $status = [int]$battery.BatteryStatus
  $percentage = if ($null -eq $battery.EstimatedChargeRemaining) {
    -1
  } else {
    [int]$battery.EstimatedChargeRemaining
  }

  @{
    hasBattery = $true
    percentage = $percentage
    charging = ($status -in @(6, 7, 8, 9, 11))
    pluggedIn = ($status -in @(2, 3, 6, 7, 8, 9, 11))
  } | ConvertTo-Json -Compress
}
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

function batteryIcon(
  percentage: number,
  charging: boolean,
  pluggedIn: boolean,
): Buffer {
  const fillColor =
    percentage <= 15
      ? "#ef4444"
      : charging
        ? "#ffffff"
        : pluggedIn
          ? "#60a5fa"
          : "#f5f5f5";

  const fillWidth = (12 * percentage) / 100;

  const chargingMark = charging
    ? `<path
        d="M10.8282 9L9.08572 11.1749C8.89899 11.4079 9.03283 11.7433 9.33733 11.8053L11.1627 12.1773C11.4873 12.2434 11.6111 12.6147 11.3842 12.8413L9.22216 15"
        stroke-linejoin="round"
      />`
    : "";

  const pluggedMark =
    pluggedIn && !charging
      ? `<g
          transform="translate(7.5 7.5) scale(.42)"
          fill="none"
          stroke="#f5f5f5"
          stroke-width="3.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M15.5 2V6M8.5 6V2" />
          <path d="M6.00446 7.61331C5.93719 6.74273 6.63957 6 7.53014 6H16.4699C17.3604 6 18.0628 6.74273 17.9955 7.61331L17.8117 9.99197C17.6796 11.7019 17.1011 13.3498 16.132 14.7773L15.5312 15.6622C14.9638 16.4979 14.0077 17 12.9838 17H11.0162C9.99228 17 9.03617 16.4979 8.46881 15.6622L7.86803 14.7773C6.89885 13.3498 6.32041 11.7019 6.18827 9.99197L6.00446 7.61331Z" />
          <path d="M12 17V22M11 9H13" />
        </g>`
      : "";

  const svg = `
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      width="24"
      height="24"
      fill="none"
      stroke="#f5f5f5"
      stroke-width="1.5"
      stroke-linecap="round"
    >
      <rect
        x="4.5"
        y="8.5"
        width="12"
        height="7"
        rx="1"
        fill="#f5f5f5"
        opacity=".18"
        stroke="none"
      />
      <rect
        x="4.5"
        y="8.5"
        width="${fillWidth}"
        height="7"
        rx="1"
        fill="${fillColor}"
        stroke="none"
      />
      ${chargingMark}
      ${pluggedMark}
      <path d="M2 12C2 9.17157 2 7.75736 2.87868 6.87868C3.75736 6 5.17157 6 8 6H13C15.8284 6 17.2426 6 18.1213 6.87868C19 7.75736 19 9.17157 19 12C19 14.8284 19 16.2426 18.1213 17.1213C17.2426 18 15.8284 18 13 18H8C5.17157 18 3.75736 18 2.87868 17.1213C2 16.2426 2 14.8284 2 12Z" />
      <path d="M19 9.5L20.0272 9.6712C20.7085 9.78475 21.0491 9.84152 21.3076 10.0067C21.5618 10.1691 21.7612 10.4044 21.8796 10.6819C22 10.964 22 11.3093 22 12C22 12.6907 22 13.036 21.8796 13.3181C21.7612 13.5956 21.5618 13.8309 21.3076 13.9933C21.0491 14.1585 20.7085 14.2153 20.0272 14.3288L19 14.5" />
    </svg>
  `;

  return Buffer.from(svg);
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

    item.setIcon(batteryIcon(percentage, charging, status.pluggedIn === true));
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
