# Removes the Barista helper service. Asks for administrator rights once.
$ErrorActionPreference = 'Stop'
$name = 'BaristaSaver'
$dir = Join-Path $env:ProgramFiles 'Barista\helper'

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
  Start-Process powershell -Verb RunAs -Wait -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
  exit
}

try {
  if (Get-Service $name -ErrorAction SilentlyContinue) {
    sc.exe stop $name | Out-Null
    Start-Sleep -Seconds 1
    sc.exe delete $name | Out-Null
  }
  Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
  Write-Host 'Removed the Barista helper.'
} catch {
  Write-Host "FAILED: $($_.Exception.Message)" -ForegroundColor Red
}
Read-Host 'Press Enter to close'
