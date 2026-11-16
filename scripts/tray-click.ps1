# Clicks a tray icon through the running bar (debug builds: `pnpm inspect --click`) and reports what
# the app did, so "behaves like the real tray" is checked on real apps (v0.7).
#   powershell -File scripts/tray-click.ps1 -Id tray/1051c-1fb859bb -Button right
# -Button left|right|middle|double   -Wait ms to look for a reaction (default 1500)
# Reports: windows that appeared (title, class, owner pid), whether a menu popup (#32768) is up and
# where its top-left is relative to the pointer, and the foreground change. Then it cleans up:
# Escape closes a menu, WM_CLOSE closes windows that appeared during the test (only those).
# Before the click it does a REAL mouse click on an empty part of the bar, like the user's own click
# on the icon: Windows only lets another app take focus (so its menu closes when you click away)
# right after real input in the bar's process. The pointer then stays on the bar: apps that ignore
# the version 4 anchor draw their menu at the pointer.
param(
  [Parameter(Mandatory)][string]$Id,
  [ValidateSet('left', 'right', 'middle', 'double')][string]$Button = 'left',
  [int]$Wait = 1500,
  [switch]$KeepOpen
)

Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public class Win {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  public class Info { public IntPtr H; public string Title, Class; public uint Pid; public RECT R; }
  public static List<Info> Visible() {
    var list = new List<Info>();
    EnumWindows((h, l) => {
      if (!IsWindowVisible(h)) return true;
      var t = new StringBuilder(256); var c = new StringBuilder(256);
      GetWindowText(h, t, 256); GetClassName(h, c, 256);
      uint pid; GetWindowThreadProcessId(h, out pid);
      RECT r; GetWindowRect(h, out r);
      list.Add(new Info { H = h, Title = t.ToString(), Class = c.ToString(), Pid = pid, R = r });
      return true;
    }, IntPtr.Zero);
    return list;
  }
}
'@
[Win]::SetProcessDPIAware() | Out-Null

function Click-Bar {
  # The bar is the wide, short "Tauri Window" at the top of a monitor. Click a third of the way along:
  # that part of the bar has no items.
  $bar = [Win]::Visible() | Where-Object { $_.Class -eq 'Tauri Window' -and ($_.R.R - $_.R.L) -gt 1000 -and ($_.R.B - $_.R.T) -lt 100 } | Select-Object -First 1
  if (-not $bar) { return $false }
  [Win]::SetCursorPos($bar.R.L + [int](($bar.R.R - $bar.R.L) / 3), $bar.R.T + [int](($bar.R.B - $bar.R.T) / 2)) | Out-Null
  Start-Sleep -Milliseconds 120
  [Win]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [Win]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 150
  return $true
}

$root = Split-Path -Parent $PSScriptRoot
if (-not (Click-Bar)) { Write-Warning 'bar window not found: clicking without real input first' }
$before = [Win]::Visible()
$beforeHandles = @{}; foreach ($w in $before) { $beforeHandles[$w.H] = $true }
$fgBefore = [Win]::GetForegroundWindow()

$null = node "$root/examples/inspect/src/index.ts" --click $Id $Button 2>&1
Start-Sleep -Milliseconds $Wait

$after = [Win]::Visible()
$new = $after | Where-Object { -not $beforeHandles.ContainsKey($_.H) }
$fgAfter = [Win]::GetForegroundWindow()
$p = New-Object Win+POINT; [Win]::GetCursorPos([ref]$p) | Out-Null

$menu = $new | Where-Object { $_.Class -eq '#32768' } | Select-Object -First 1
$windows = $new | Where-Object { $_.Class -ne '#32768' -and ($_.Title -or $_.Class -match 'Window|Frame|Chrome|Qt|Xaml|WinUI') }
"[$Id $Button]"
if ($menu) {
  "  menu: yes, top-left ($($menu.R.L),$($menu.R.T)) pointer ($($p.X),$($p.Y)) size $($menu.R.R - $menu.R.L)x$($menu.R.B - $menu.R.T)"
} else { '  menu: no' }
foreach ($w in $windows | Select-Object -First 4) {
  $name = (Get-Process -Id $w.Pid -ErrorAction SilentlyContinue).ProcessName
  "  window: '$($w.Title)' [$($w.Class)] $name at ($($w.R.L),$($w.R.T)) $($w.R.R - $w.R.L)x$($w.R.B - $w.R.T)"
}
if ($fgAfter -ne $fgBefore) {
  $fp = 0; [Win]::GetWindowThreadProcessId($fgAfter, [ref]$fp) | Out-Null
  "  foreground: now $((Get-Process -Id $fp -ErrorAction SilentlyContinue).ProcessName)"
}
if (-not $menu -and -not $windows -and $fgAfter -eq $fgBefore) { '  nothing visible happened' }

if (-not $KeepOpen) {
  if ($menu) {
    [Win]::keybd_event(0x1B, 0, 0, [UIntPtr]::Zero); [Win]::keybd_event(0x1B, 0, 2, [UIntPtr]::Zero); Start-Sleep -Milliseconds 300
    # Escape goes to the focused window. If the menu never got focus, post Escape and WM_CANCELMODE
    # straight to the menu window (this is test cleanup only; a user just clicks away).
    foreach ($m in [Win]::Visible() | Where-Object { $_.Class -eq '#32768' }) {
      [Win]::PostMessage($m.H, 0x100, [IntPtr]0x1B, [IntPtr]::Zero) | Out-Null
      [Win]::PostMessage($m.H, 0x101, [IntPtr]0x1B, [IntPtr]::Zero) | Out-Null
      [Win]::PostMessage($m.H, 0x1F, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    }
    Start-Sleep -Milliseconds 300
  }
  foreach ($w in $windows) { [Win]::PostMessage($w.H, 0x10, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null }  # WM_CLOSE
  Start-Sleep -Milliseconds 300
  if ($menu) {
    $still = [Win]::Visible() | Where-Object { $_.Class -eq '#32768' }
    "  menu after Escape: $(if ($still) { 'STILL OPEN' } else { 'closed' })"
  }
}
