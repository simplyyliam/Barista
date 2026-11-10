# Helper for pnpm bench: times "click -> menu window visible". For each line "show" or "hide" on
# stdin it starts a stopwatch, waits until the bar's menu window is (in)visible, and prints the
# milliseconds. The time includes the bar handling the click and Windows showing the window
# (not React painting it), plus about a millisecond of pipe overhead.
Add-Type @"
using System; using System.Runtime.InteropServices;
public class MenuWin {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindowW(string c, string t);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
}
"@
$hwnd = [MenuWin]::FindWindowW("Tauri Window", "Barista menu")
if ($hwnd -eq [IntPtr]::Zero) { "no-window"; exit 1 }
"ready"
while ($null -ne ($line = [Console]::In.ReadLine())) {
  $want = $line -eq "show"
  $sw = [Diagnostics.Stopwatch]::StartNew()
  # The bench clicks only after this line, so the time can only be a little too long, never short.
  [Console]::Out.WriteLine("armed"); [Console]::Out.Flush()
  while ([MenuWin]::IsWindowVisible($hwnd) -ne $want -and $sw.ElapsedMilliseconds -lt 2000) { }
  [Console]::Out.WriteLine($sw.Elapsed.TotalMilliseconds.ToString("F1", [Globalization.CultureInfo]::InvariantCulture))
  [Console]::Out.Flush()
}
