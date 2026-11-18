# Memory of the release bar (private working set of the bar and its WebView2 children, the same number
# `pnpm bench` reports), with and without the settings window.
#   powershell -File scripts/settings-memory.ps1            bar alone
#   powershell -File scripts/settings-memory.ps1 -Open      bar + settings window (BARISTA_OPEN_SETTINGS)
# Close the bar (and pnpm dev) first. Prints one line per sample, then stops the bar.
param([switch]$Open, [int]$Settle = 20, [int]$Samples = 3, [int]$Gap = 8)

$exe = Join-Path $PSScriptRoot "..\target\release\barista.exe"
if (Get-Process barista -ErrorAction SilentlyContinue) { Write-Error "barista.exe is already running"; exit 1 }
if ($Open) { $env:BARISTA_OPEN_SETTINGS = "1" } else { Remove-Item Env:BARISTA_OPEN_SETTINGS -ErrorAction SilentlyContinue }
$bar = Start-Process -FilePath $exe -PassThru
Start-Sleep -Seconds $Settle

function Measure-Tree($root) {
  $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId
  $ids = [System.Collections.Generic.HashSet[int]]::new(); [void]$ids.Add($root)
  do { $n = $ids.Count
    foreach ($p in $all) { if ($ids.Contains([int]$p.ParentProcessId)) { [void]$ids.Add([int]$p.ProcessId) } }
  } while ($ids.Count -ne $n)
  $perf = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object { $ids.Contains([int]$_.IDProcess) }
  [pscustomobject]@{
    PrivateMB = [math]::Round(($perf | Measure-Object -Property WorkingSetPrivate -Sum).Sum / 1MB, 1)
    Processes = @($perf).Count
  }
}

1..$Samples | ForEach-Object {
  $m = Measure-Tree $bar.Id
  "{0}: {1} MB private, {2} processes" -f ($(if ($Open) { "settings open" } else { "bar alone" })), $m.PrivateMB, $m.Processes
  Start-Sleep -Seconds $Gap
}
Stop-Process -Id $bar.Id -Force -ErrorAction SilentlyContinue
