# Times a popover with a REAL mouse click: from the click until the app's window is visible, and
# until it also has the keyboard focus. Needs the bar and an app with a popover window running
# (examples/test-client with start:electron, examples/quick-note or examples/spotify).
#   powershell -File scripts/popover-latency.ps1 -ProcessId <app pid> -X <icon x> -Y <icon y> [-Rounds 10]
# X and Y are physical screen pixels over the app's icon in the bar. The click is real input: the
# bar can only give another program focus after real input (a click faked over the pipe cannot).
param(
  [Parameter(Mandatory = $true)][int]$ProcessId,
  [Parameter(Mandatory = $true)][int]$X,
  [Parameter(Mandatory = $true)][int]$Y,
  [int]$Rounds = 10,
  [int]$SettleMs = 800
)

Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public class PopLat {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  public static uint ForegroundPid() { uint p; GetWindowThreadProcessId(GetForegroundWindow(), out p); return p; }
  // A visible, non-tiny window of `pid`: the popover.
  public static bool Visible(uint pid) {
    bool found = false;
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      RECT r;
      if (p == pid && IsWindowVisible(h) && GetWindowRect(h, out r) && (r.R - r.L) > 40 && (r.B - r.T) > 40) found = true;
      return true;
    }, IntPtr.Zero);
    return found;
  }
  // The foreground window IS a visible, non-tiny window of `pid`: the popover has the focus.
  public static bool PopoverHasFocus(uint pid) {
    IntPtr h = GetForegroundWindow();
    uint p; GetWindowThreadProcessId(h, out p);
    RECT r;
    return p == pid && IsWindowVisible(h) && GetWindowRect(h, out r) && (r.R - r.L) > 40 && (r.B - r.T) > 40;
  }
  public static void Click() { mouse_event(2, 0, 0, 0, UIntPtr.Zero); mouse_event(4, 0, 0, 0, UIntPtr.Zero); }
}
"@
[void][PopLat]::SetProcessDPIAware()
$pidU = [uint32]$ProcessId
# The bar's page needs a few mouse moves over it before the first click counts as a click on the icon.
foreach ($dx in 4, -3, 2, 0) { [void][PopLat]::SetCursorPos($X + $dx, $Y); Start-Sleep -Milliseconds 80 }
Start-Sleep -Milliseconds 300

function CloseIfOpen {
  for ($i = 0; $i -lt 3 -and [PopLat]::Visible($pidU); $i++) { [PopLat]::Click(); Start-Sleep -Milliseconds $SettleMs }
}

$visible = @()
$focused = @()
$skipped = 0
for ($n = 0; $n -lt $Rounds; $n++) {
  CloseIfOpen
  if ([PopLat]::Visible($pidU)) { Write-Output "round $($n + 1): could not start closed, skipped"; $skipped++; continue }
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  [PopLat]::Click()
  $v = $null; $f = $null
  while ($sw.ElapsedMilliseconds -lt 2000 -and ($null -eq $v -or $null -eq $f)) {
    if ($null -eq $v -and [PopLat]::Visible($pidU)) { $v = $sw.Elapsed.TotalMilliseconds }
    if ($null -eq $f -and [PopLat]::PopoverHasFocus($pidU)) { $f = $sw.Elapsed.TotalMilliseconds }
  }
  if ($null -ne $v) { $visible += $v }
  if ($null -ne $f) { $focused += $f }
  Write-Output ("round {0}: visible {1} ms, focused {2} ms" -f ($n + 1), $(if ($null -eq $v) { "never" } else { [int]$v }), $(if ($null -eq $f) { "never" } else { [int]$f }))
  Start-Sleep -Milliseconds $SettleMs
}
CloseIfOpen
Write-Output "rounds that never opened: $($Rounds - $visible.Count - $skipped)"

function Stat($values) {
  if ($values.Count -eq 0) { return "none" }
  $sorted = $values | Sort-Object
  "median {0} / worst {1} ms (of {2})" -f [int]$sorted[[int][math]::Floor($sorted.Count / 2)], [int]$sorted[-1], $values.Count
}
Write-Output ("click -> window visible : " + (Stat $visible))
Write-Output ("click -> keyboard focus : " + (Stat $focused))
