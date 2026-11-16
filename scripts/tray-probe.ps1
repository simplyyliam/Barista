# Click probe for the tray (v0.7). Adds two raw tray icons and logs every callback message the owner
# window gets, so we can compare what Explorer sends with what the bar sends.
#   icon 1: plain (version 0): wParam = icon id, lParam = mouse message
#   icon 2: NIM_SETVERSION 4:  wParam = anchor x,y, lParam = message (low) + icon id (high)
# Each line in -Log: "<unix ms> id=<n> <MESSAGE> wParam=<hex> lParam=<hex> cursor=<x>,<y>"
# -Menu shows a small popup menu on right click like a real app does (SetForegroundWindow + TrackPopupMenu)
# and logs which entry was picked ("menu <id> picked <cmd>", "menu <id> closed").
# -Seconds N quits after N seconds (default 120; 0 = no limit). Ctrl+C does not stop it (it sits in a
# message loop): create scripts/probe.stop to stop it at once. Quitting removes the icons.
#
#   powershell -File scripts/tray-probe.ps1 -Log probe.log -Menu -Seconds 120
param(
  [string]$Log = '',
  [switch]$Menu,
  [int]$Seconds = 120
)

Add-Type -ReferencedAssemblies System.Windows.Forms, System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.IO;
using System.Runtime.InteropServices;
using System.Windows.Forms;

public class ProbeWindow : NativeWindow {
  const uint CALLBACK = 0x8001; // WM_APP + 1
  [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
  struct NID {
    public uint cbSize; public IntPtr hWnd; public uint uID; public uint uFlags; public uint uCallbackMessage;
    public IntPtr hIcon;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string szTip;
    public uint dwState; public uint dwStateMask;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string szInfo;
    public uint uVersion;
    [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 64)] public string szInfoTitle;
    public uint dwInfoFlags; public Guid guidItem; public IntPtr hBalloonIcon;
  }
  [StructLayout(LayoutKind.Sequential)] struct PT { public int x, y; }
  [DllImport("shell32.dll", CharSet = CharSet.Unicode)] static extern bool Shell_NotifyIconW(uint msg, ref NID data);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern uint RegisterWindowMessageW(string name);
  [DllImport("user32.dll")] static extern bool GetCursorPos(out PT p);
  [DllImport("user32.dll")] static extern IntPtr CreatePopupMenu();
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern bool AppendMenuW(IntPtr m, uint flags, UIntPtr id, string text);
  [DllImport("user32.dll")] static extern int TrackPopupMenu(IntPtr m, uint flags, int x, int y, int r, IntPtr hwnd, IntPtr rect);
  [DllImport("user32.dll")] static extern bool DestroyMenu(IntPtr m);
  [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] static extern bool PostMessageW(IntPtr h, uint msg, IntPtr w, IntPtr l);

  readonly string log; readonly bool menu; readonly uint taskbarCreated; readonly Icon[] icons = new Icon[3];
  public ProbeWindow(string log, bool menu) {
    this.log = log; this.menu = menu;
    taskbarCreated = RegisterWindowMessageW("TaskbarCreated");
    CreateHandle(new CreateParams { Caption = "barista-probe", ClassName = "STATIC" });
    for (int i = 1; i <= 2; i++) icons[i] = Draw(i);
    AddAll();
  }
  static Icon Draw(int n) {
    var bmp = new Bitmap(32, 32);
    using (var g = Graphics.FromImage(bmp)) {
      g.FillEllipse(n == 1 ? Brushes.DodgerBlue : Brushes.Orange, 2, 2, 28, 28);
      g.DrawString(n.ToString(), new Font("Segoe UI", 11, FontStyle.Bold, GraphicsUnit.Pixel), Brushes.White, 10, 8);
    }
    return Icon.FromHandle(bmp.GetHicon());
  }
  NID Data(uint id, uint flags) {
    return new NID { cbSize = (uint)Marshal.SizeOf(typeof(NID)), hWnd = Handle, uID = id, uFlags = flags,
      uCallbackMessage = CALLBACK, hIcon = icons[id].Handle, szTip = "Barista probe " + id };
  }
  void AddAll() {
    for (uint id = 1; id <= 2; id++) {
      var d = Data(id, 7); Shell_NotifyIconW(0, ref d);
      if (id == 2) { d.uVersion = 4; Shell_NotifyIconW(4, ref d); }
    }
    Stamp("added");
  }
  public void RemoveAll() {
    for (uint id = 1; id <= 2; id++) { var d = Data(id, 0); Shell_NotifyIconW(2, ref d); }
    Stamp("removed");
  }
  public void Stamp(string what) {
    if (string.IsNullOrEmpty(log)) return;
    File.AppendAllText(log, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() + " " + what + "\n");
  }
  static string Name(uint m) {
    switch (m) {
      case 0x200: return "WM_MOUSEMOVE"; case 0x201: return "WM_LBUTTONDOWN"; case 0x202: return "WM_LBUTTONUP";
      case 0x203: return "WM_LBUTTONDBLCLK"; case 0x204: return "WM_RBUTTONDOWN"; case 0x205: return "WM_RBUTTONUP";
      case 0x206: return "WM_RBUTTONDBLCLK"; case 0x207: return "WM_MBUTTONDOWN"; case 0x208: return "WM_MBUTTONUP";
      case 0x209: return "WM_MBUTTONDBLCLK"; case 0x7B: return "WM_CONTEXTMENU";
      case 0x400: return "NIN_SELECT"; case 0x401: return "NIN_KEYSELECT";
      case 0x402: return "NIN_BALLOONSHOW"; case 0x403: return "NIN_BALLOONHIDE";
      case 0x404: return "NIN_BALLOONTIMEOUT"; case 0x405: return "NIN_BALLOONUSERCLICK";
      case 0x406: return "NIN_POPUPOPEN"; case 0x407: return "NIN_POPUPCLOSE";
      default: return "0x" + m.ToString("X");
    }
  }
  protected override void WndProc(ref Message m) {
    if (m.Msg == taskbarCreated) { AddAll(); }
    else if (m.Msg == CALLBACK) {
      long w = m.WParam.ToInt64(), l = m.LParam.ToInt64();
      PT c; GetCursorPos(out c);
      // Version 4 packs the message and the id into lParam, and the anchor into wParam.
      bool v4 = (l >> 16) != 0;   // version 0 has a bare message number here
      uint msg = v4 ? (uint)(l & 0xFFFF) : (uint)l;
      uint id = v4 ? (uint)((l >> 16) & 0xFFFF) : (uint)w;
      string anchor = v4 ? " anchor=" + (short)(w & 0xFFFF) + "," + (short)((w >> 16) & 0xFFFF) : "";
      Stamp("id=" + id + " " + Name(msg) + " wParam=0x" + w.ToString("X") + " lParam=0x" + l.ToString("X") +
            " cursor=" + c.x + "," + c.y + anchor);
      bool open = menu && ((id == 1 && msg == 0x205) || (id == 2 && msg == 0x7B));
      if (open) ShowMenu(id, c);
    }
    base.WndProc(ref m);
  }
  void ShowMenu(uint id, PT at) {
    var hmenu = CreatePopupMenu();
    AppendMenuW(hmenu, 0, (UIntPtr)1, "Probe entry one");
    AppendMenuW(hmenu, 0, (UIntPtr)2, "Probe entry two");
    SetForegroundWindow(Handle);                 // without it the menu would not close on an outside click
    int cmd = TrackPopupMenu(hmenu, 0x100 /*TPM_RETURNCMD*/, at.x, at.y, 0, Handle, IntPtr.Zero);
    PostMessageW(Handle, 0, IntPtr.Zero, IntPtr.Zero); // the documented WM_NULL workaround
    Stamp("menu " + id + (cmd == 0 ? " closed" : " picked " + cmd));
    DestroyMenu(hmenu);
  }
}
'@

$window = New-Object ProbeWindow $Log, $Menu.IsPresent
"probe running (pid $PID): 2 icons, Ctrl+C to stop"

if ($Seconds -gt 0) {
  $quit = New-Object System.Windows.Forms.Timer
  $quit.Interval = $Seconds * 1000
  $quit.Add_Tick({ [System.Windows.Forms.Application]::ExitThread() })
  $quit.Start()
}
# Ctrl+C cannot interrupt the message loop, so also stop as soon as the file "probe.stop" appears
# next to the script: `New-Item scripts/probe.stop` (or `echo > scripts/probe.stop`).
$stopFile = Join-Path $PSScriptRoot 'probe.stop'
Remove-Item $stopFile -ErrorAction SilentlyContinue
$watch = New-Object System.Windows.Forms.Timer
$watch.Interval = 500
$watch.Add_Tick({ if (Test-Path $stopFile) { Remove-Item $stopFile -ErrorAction SilentlyContinue; [System.Windows.Forms.Application]::ExitThread() } })
$watch.Start()
"stops by itself after $Seconds s, or create scripts/probe.stop to stop it now"
try { [System.Windows.Forms.Application]::Run() }
finally { $window.RemoveAll(); 'removed the tray icons' }
