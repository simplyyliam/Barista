# Polls the top-level windows of one process and the foreground window, every few ms, and prints a
# timeline. Used to time a popover (appears, gets focus) and to check where it sits.
#   powershell -File scripts/popover-visible.ps1 -ProcessId 1234 [-Seconds 3] [-IntervalMs 5]
# Prints one line per change: "<ms since start> visible=<n> foreground=<ours|other:pid> rects=...".
# Start it BEFORE the click; it stamps t=0 when it starts and prints "ready" first.
param(
  [Parameter(Mandatory = $true)][int]$ProcessId,
  [double]$Seconds = 3,
  [int]$IntervalMs = 5
)

Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class PopWin {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public static uint ForegroundPid() { uint p; GetWindowThreadProcessId(GetForegroundWindow(), out p); return p; }
  // Visible windows of `pid` that are not tiny, as "L,T,WxH" strings.
  public static List<string> Visible(uint pid) {
    var found = new List<string>();
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      RECT r;
      if (p == pid && IsWindowVisible(h) && GetWindowRect(h, out r) && (r.R - r.L) > 40 && (r.B - r.T) > 40)
        found.Add(r.L + "," + r.T + "," + (r.R - r.L) + "x" + (r.B - r.T));
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
"@

[void][PopWin]::SetProcessDPIAware()
$sw = [System.Diagnostics.Stopwatch]::StartNew()
Write-Output "ready"
$last = ""
while ($sw.Elapsed.TotalSeconds -lt $Seconds) {
  $rects = [PopWin]::Visible([uint32]$ProcessId)
  $fg = [PopWin]::ForegroundPid()
  $who = if ($fg -eq $ProcessId) { "ours" } else { "other:$fg" }
  $line = "visible=$($rects.Count) foreground=$who rects=$($rects -join ';')"
  if ($line -ne $last) {
    Write-Output ("{0,6} ms  {1}" -f [int]$sw.Elapsed.TotalMilliseconds, $line)
    $last = $line
  }
  Start-Sleep -Milliseconds $IntervalMs
}
