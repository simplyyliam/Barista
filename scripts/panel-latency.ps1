# Times "openPanel -> panel window visible" over the bar's named pipe (debug builds of the bar, or
# BARISTA_DEBUG set). Usage: powershell -File scripts/panel-latency.ps1 [anchor item id] [runs]
# Like the menu bench, the time covers the bar handling the message and Windows showing the window
# (not React painting it). Without the UI measuring itself the bar shows the window after its 30 ms
# fallback, so this is the worst case; with panelResized it is faster.
param([string]$Anchor = "s1/none", [int]$Runs = 10)
$ErrorActionPreference = "Stop"
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Text;
public class PanelWin {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  public static bool Visible() {
    bool any = false;
    EnumWindows((h, l) => {
      var sb = new StringBuilder(64); GetWindowText(h, sb, 64);
      if (sb.ToString() == "Barista panel" && IsWindowVisible(h)) any = true;
      return true;
    }, IntPtr.Zero);
    return any;
  }
}
"@
$pipe = New-Object System.IO.Pipes.NamedPipeClientStream(".", "barista", [System.IO.Pipes.PipeDirection]::InOut)
$pipe.Connect(3000)
function Send($obj) {
  $json = [Text.Encoding]::UTF8.GetBytes(($obj | ConvertTo-Json -Compress))
  $head = [BitConverter]::GetBytes([uint32]$json.Length)
  $pipe.Write($head, 0, 4); $pipe.Write($json, 0, $json.Length); $pipe.Flush()
}
Send @{ type = "hello"; protocolVersion = "0.4"; appName = "panel-latency"; pid = $PID; sdk = @{ name = "ps"; version = "0" } }
$buf = New-Object byte[] 4; [void]$pipe.Read($buf, 0, 4)
$body = New-Object byte[] ([BitConverter]::ToUInt32($buf, 0)); [void]$pipe.Read($body, 0, $body.Length)
$times = @()
for ($i = 0; $i -lt $Runs; $i++) {
  $sw = [Diagnostics.Stopwatch]::StartNew()
  Send @{ type = "debug.openPanel"; name = "test"; anchor = $Anchor; width = 320; height = 240 }
  while (-not [PanelWin]::Visible() -and $sw.ElapsedMilliseconds -lt 2000) { }
  $times += $sw.Elapsed.TotalMilliseconds
  Send @{ type = "debug.closePanel" }
  while ([PanelWin]::Visible() -and $sw.ElapsedMilliseconds -lt 4000) { }
  Start-Sleep -Milliseconds 300
}
$pipe.Dispose()
$sorted = $times | Sort-Object
"runs: " + ($times | ForEach-Object { $_.ToString("F1") }) -join " "
"median: {0:F1} ms   max: {1:F1} ms" -f $sorted[[int]($sorted.Count / 2)], $sorted[-1]
