# System monitor for the Barista menu bar: CPU, RAM, disk C: and the busiest apps.
# Drop this file in %APPDATA%\Barista\scripts. The "5s" in the name is how often it runs.
# Output format: first line = bar label, "---" starts the menu, "--" makes a submenu.
# PowerShell takes a few hundred ms to start, so 5s is about the fastest that makes sense here.

$ErrorActionPreference = 'Stop'
$cores = [Environment]::ProcessorCount

# Two snapshots of every process's CPU time, 400 ms apart: the difference is the load right now.
function Get-CpuTimes {
    $times = @{}
    # .CPU is $null for protected processes. Reading TotalProcessorTime would throw for each of them, which is slow.
    foreach ($p in Get-Process) {
        $times[$p.Id] = @{ Name = $p.ProcessName; Cpu = [double]$p.CPU; Mem = $p.WorkingSet64 }
    }
    $times
}
$before = Get-CpuTimes
Start-Sleep -Milliseconds 400
$after = Get-CpuTimes

# Several processes can share a name (chrome), so add them up per name.
$apps = @{}
foreach ($id in $after.Keys) {
    $now = $after[$id]
    $cpu = 0.0
    if ($before.ContainsKey($id)) { $cpu = ($now.Cpu - $before[$id].Cpu) / 0.4 / $cores * 100 }
    $name = $now.Name -replace '\|', '/'
    if (-not $apps.ContainsKey($name)) { $apps[$name] = @{ Cpu = 0.0; Mem = 0L } }
    $apps[$name].Cpu += $cpu
    $apps[$name].Mem += $now.Mem
}

# Memory and disk through .NET: the WMI/CIM versions (Get-CimInstance, Get-PSDrive) cost 0.2 to 1 s each.
Add-Type -AssemblyName Microsoft.VisualBasic
$mem = New-Object Microsoft.VisualBasic.Devices.ComputerInfo
$totalGb = $mem.TotalPhysicalMemory / 1GB
$usedGb = ($mem.TotalPhysicalMemory - $mem.AvailablePhysicalMemory) / 1GB
$ramPct = [math]::Round($usedGb / $totalGb * 100)
# Total CPU is the busy time of all processes added up, so no extra query is needed.
$cpuPct = [math]::Min(100, [math]::Round(($apps.Values | ForEach-Object { $_.Cpu } | Measure-Object -Sum).Sum))
$disk = [IO.DriveInfo]::new('C')
$diskPct = [math]::Round(($disk.TotalSize - $disk.AvailableFreeSpace) / $disk.TotalSize * 100)

# Red when something is above 90%.
$label = "CPU $cpuPct%  RAM $ramPct%"
if ($cpuPct -gt 90 -or $ramPct -gt 90) { $label += ' | color=red' }
$label
'---'

$ramLine = ('Memory: {0:N1} of {1:N1} GB' -f $usedGb, $totalGb)
if ($ramPct -gt 90) { $ramLine += ' | color=red' } elseif ($ramPct -gt 75) { $ramLine += ' | color=orange' }
$ramLine
$diskLine = "Disk C: $diskPct% used"
if ($diskPct -gt 90) { $diskLine += ' | color=red' }
$diskLine
'---'

'Top apps by CPU'
$apps.GetEnumerator() | Sort-Object { $_.Value.Cpu } -Descending | Select-Object -First 5 | ForEach-Object {
    '--{0} {1:N0}%' -f $_.Key, $_.Value.Cpu
}
'Top apps by memory'
$apps.GetEnumerator() | Sort-Object { $_.Value.Mem } -Descending | Select-Object -First 5 | ForEach-Object {
    '--{0} {1:N1} GB' -f $_.Key, ($_.Value.Mem / 1GB)
}
'---'
'Open Task Manager | shell=taskmgr'
'Open the docs | href=https://example.com'
'Refresh now | refresh=true'
