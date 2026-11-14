# Soak test: samples the running bar every few minutes (CPU over a short window, private memory,
# process count, whether it is still alive) and appends a line to a log. Start the bar yourself
# with your normal tray apps running, then:
#
#   powershell -File scripts/soak.ps1 -Minutes 60 -EveryMinutes 5 -Log soak.log
param([int]$Minutes = 60, [int]$EveryMinutes = 5, [int]$WindowSeconds = 30, [string]$Log = 'soak.log')

$cores = [Environment]::ProcessorCount
function Tree($rootId) {
  $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId
  $ids = New-Object System.Collections.Generic.HashSet[int]; [void]$ids.Add($rootId)
  do { $n = $ids.Count; foreach ($p in $all) { if ($ids.Contains([int]$p.ParentProcessId)) { [void]$ids.Add([int]$p.ProcessId) } } } while ($ids.Count -ne $n)
  $ids
}
function Cpu($ids) { $t = 0.0; foreach ($id in $ids) { $p = Get-Process -Id $id -ErrorAction SilentlyContinue; if ($p) { $t += $p.CPU } }; $t }

$end = (Get-Date).AddMinutes($Minutes)
"time,alive,processes,cpu_percent,private_mb,explorer_pid" | Out-File $Log -Encoding utf8
while ((Get-Date) -lt $end) {
  $bar = Get-Process barista -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $bar) {
    "$((Get-Date).ToString('s')),False,0,,," | Add-Content $Log
  } else {
    $ids = Tree $bar.Id
    $a = Cpu $ids; Start-Sleep -Seconds $WindowSeconds; $b = Cpu $ids
    $cpu = [Math]::Round((($b - $a) / $WindowSeconds / $cores) * 100, 3)
    $private = (Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object { $ids.Contains([int]$_.IDProcess) } | Measure-Object -Property WorkingSetPrivate -Sum).Sum / 1MB
    $explorer = (Get-Process explorer -ErrorAction SilentlyContinue | Select-Object -First 1).Id
    "$((Get-Date).ToString('s')),True,$($ids.Count),$cpu,$([Math]::Round($private,0)),$explorer" | Add-Content $Log
  }
  Start-Sleep -Seconds ([Math]::Max(1, $EveryMinutes * 60 - $WindowSeconds))
}
