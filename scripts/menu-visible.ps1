# Helper for pnpm bench: times "click -> menu window visible". For each line "show" or "hide" on
# stdin it starts a stopwatch, waits until the bar's menu window is (in)visible, and prints the
# milliseconds. The time includes the bar handling the click and Windows showing the window
# (not React painting it), plus about a millisecond of pipe overhead.
#
# The menu and its submenu are two windows with the same title. FindWindow returns whichever is on
# top, which can be the hidden submenu one, so the check looks at every window with that title: the
# menu is "visible" when any of them is (the submenu only opens on hover, never in the bench).
Add-Type @"
using System; using System.Runtime.InteropServices; using System.Text;
public class MenuWin {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  public static int Count() {
    int count = 0;
    EnumWindows((h, l) => {
      var sb = new StringBuilder(64); GetWindowText(h, sb, 64);
      if (sb.ToString() == "Barista menu") count++;
      return true;
    }, IntPtr.Zero);
    return count;
  }
  public static bool AnyVisible() {
    bool any = false;
    EnumWindows((h, l) => {
      var sb = new StringBuilder(64); GetWindowText(h, sb, 64);
      if (sb.ToString() == "Barista menu" && IsWindowVisible(h)) any = true;
      return true;
    }, IntPtr.Zero);
    return any;
  }
}
"@
if ([MenuWin]::Count() -eq 0) { "no-window"; exit 1 }
"ready"
while ($null -ne ($line = [Console]::In.ReadLine())) {
  $want = $line -eq "show"
  $sw = [Diagnostics.Stopwatch]::StartNew()
  # The bench clicks only after this line, so the time can only be a little too long, never short.
  [Console]::Out.WriteLine("armed"); [Console]::Out.Flush()
  while ([MenuWin]::AnyVisible() -ne $want -and $sw.ElapsedMilliseconds -lt 2000) { }
  [Console]::Out.WriteLine($sw.Elapsed.TotalMilliseconds.ToString("F1", [Globalization.CultureInfo]::InvariantCulture))
  [Console]::Out.Flush()
}
