# Installs the Barista helper service (energy saver switch). Asks for administrator rights once.
# Usage: pnpm helper:install   (builds the helper first)
param([string]$Exe = (Join-Path $PSScriptRoot '..\..\target\release\barista-helper.exe'))

$ErrorActionPreference = 'Stop'
$name = 'BaristaSaver'
$dir = Join-Path $env:ProgramFiles 'Barista\helper'

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
  $full = (Resolve-Path $Exe).Path
  Start-Process powershell -Verb RunAs -Wait -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Exe `"$full`""
  exit
}

try {
  if (-not (Test-Path $Exe)) { throw "helper exe not found: $Exe (run: cargo build -p barista-saver-helper --release)" }

  # Program Files is writable by administrators only. A service must never run a file a normal user
  # can replace, so the exe is copied here and nowhere else.
  if (Get-Service $name -ErrorAction SilentlyContinue) {
    sc.exe stop $name | Out-Null
    Start-Sleep -Seconds 1
    sc.exe delete $name | Out-Null
    Start-Sleep -Seconds 1
  }
  New-Item -ItemType Directory -Force $dir | Out-Null
  Copy-Item $Exe (Join-Path $dir 'barista-helper.exe') -Force
  $bin = Join-Path $dir 'barista-helper.exe'

  sc.exe create $name binPath= "`"$bin`"" start= auto obj= LocalSystem DisplayName= "Barista helper (energy saver)" | Out-Null
  sc.exe description $name "Lets the Barista bar switch energy saver. Only sets one registry value." | Out-Null
  # Who may do what: SYSTEM and administrators everything usual; signed-in users may only ask for the
  # status and send the two control codes (LC query, LO interrogate, CR user-defined control, RC read).
  sc.exe sdset $name "D:(A;;CCLCSWRPWPDTLOCRRC;;;SY)(A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;BA)(A;;LCLOCRRC;;;IU)" | Out-Null
  sc.exe start $name | Out-Null
  Start-Sleep -Seconds 1
  Get-Service $name | Format-Table Name, Status, StartType -AutoSize | Out-String | Write-Host
  Write-Host "Installed. Restart the bar; the Battery Saver tile can switch energy saver now."
} catch {
  Write-Host "FAILED: $($_.Exception.Message)" -ForegroundColor Red
}
Read-Host 'Press Enter to close'
