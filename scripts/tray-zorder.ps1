# Measures how often Explorer's taskbar window ends up above the bar's tray window (ADR 0026).
# While it samples FindWindow("Shell_TrayWnd") every 50 ms, it does the things that make Explorer
# raise its taskbar: Start, Win+D (show desktop), Win+B / Win+T (focus the tray / taskbar buttons),
# Alt+Tab. Run with the bar running. It touches nothing but the keyboard.
#
#   powershell -File scripts/tray-zorder.ps1 [-Rounds 3]
param([int]$Rounds = 3)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class ZA {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  public static uint FirstOwner() {
    uint pid; GetWindowThreadProcessId(FindWindow("Shell_TrayWnd", null), out pid); return pid;
  }
}
"@

$bar = (Get-Process barista -ErrorAction Stop | Select-Object -First 1).Id
$explorer = [ZA]::FirstOwner()
"bar pid $bar; first Shell_TrayWnd owner now: $explorer"

function Key([byte[]]$keys) {
  foreach ($k in $keys) { [ZA]::keybd_event($k, 0, 0, [UIntPtr]::Zero) }
  Start-Sleep -Milliseconds 60
  foreach ($k in ($keys | Sort-Object -Descending)) { [ZA]::keybd_event($k, 0, 2, [UIntPtr]::Zero) }
}
$WIN = 0x5B; $ESC = 0x1B; $D = 0x44; $B = 0x42; $T = 0x54; $ALT = 0x12; $TAB = 0x09

# Each step is something that should make Explorer raise its taskbar. After each one we wait a
# moment and look; "lost" = how long Explorer was first.
$steps = @(
  @{ n = 'Start (Win)';        a = { Key @($WIN) };        undo = { Key @($ESC) } },
  @{ n = 'Show desktop (Win+D)'; a = { Key @($WIN, $D) };   undo = { Key @($WIN, $D) } },
  @{ n = 'Focus tray (Win+B)';  a = { Key @($WIN, $B) };   undo = { Key @($ESC) } },
  @{ n = 'Focus taskbar (Win+T)'; a = { Key @($WIN, $T) }; undo = { Key @($ESC) } },
  @{ n = 'Alt+Tab';            a = { Key @($ALT, $TAB) };  undo = { } }
)

$results = @()
foreach ($step in $steps) {
  $lost = 0; $samples = 0; $worst = 0; $streak = 0; $regain = 0
  for ($r = 0; $r -lt $Rounds; $r++) {
    & $step.a
    $end = (Get-Date).AddMilliseconds(1200)
    while ((Get-Date) -lt $end) {
      $owner = [ZA]::FirstOwner(); $samples++
      if ($owner -ne $bar) { $lost++; $streak++; if ($streak -gt $worst) { $worst = $streak } } else { $streak = 0 }
      Start-Sleep -Milliseconds 50
    }
    & $step.undo
    # How long until the bar is first again after the step ended?
    $t0 = Get-Date
    while ([ZA]::FirstOwner() -ne $bar -and ((Get-Date) - $t0).TotalMilliseconds -lt 3000) { Start-Sleep -Milliseconds 10 }
    $ms = [int]((Get-Date) - $t0).TotalMilliseconds
    if ($ms -gt $regain) { $regain = $ms }
    Start-Sleep -Milliseconds 500
    $streak = 0
  }
  $results += [pscustomobject]@{ Step = $step.n; Samples = $samples; 'Explorer first' = $lost; 'Worst streak (ms)' = $worst * 50; 'Back on top after (ms, worst)' = $regain }
}
$results | Format-Table -AutoSize
$final = [ZA]::FirstOwner()
"after all steps the bar is first: $($final -eq $bar)"
