# A fake tray app for testing the bar (v0.6). Adds -Count tray icons, each with its own picture and
# the tooltip "Barista test <n>". Optional live changes on icon 1:
#   -Blink      swap its picture between two every 2 s (like an unread badge)
#   -Tip        change its tooltip every 2 s to "Barista test 1 #<counter>"
# -Log <file> appends "<unix ms> <what>" at the moment each change is sent, for latency checks.
# -Seconds N  quits after N seconds (default: run until Ctrl+C). Quitting removes the icons.
#
#   powershell -File scripts/tray-icons.ps1 -Count 15 -Tip -Log tray.log -Seconds 120
param(
  [int]$Count = 15,
  [switch]$Blink,
  [switch]$Tip,
  [string]$Log = '',
  [int]$Seconds = 0
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

function New-Picture([int]$n, [bool]$alt) {
  $bmp = New-Object System.Drawing.Bitmap 32, 32
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = 'AntiAlias'
  $hue = ($n * 47) % 360
  $r = [Math]::Abs([Math]::Sin($hue * [Math]::PI / 180)) * 200 + 40
  $gr = [Math]::Abs([Math]::Sin(($hue + 120) * [Math]::PI / 180)) * 200 + 40
  $b = [Math]::Abs([Math]::Sin(($hue + 240) * [Math]::PI / 180)) * 200 + 40
  $color = [System.Drawing.Color]::FromArgb(255, [int]$r, [int]$gr, [int]$b)
  $g.FillEllipse((New-Object System.Drawing.SolidBrush $color), 2, 2, 28, 28)
  if ($alt) { $g.FillEllipse([System.Drawing.Brushes]::Red, 18, 0, 14, 14) }   # a "badge"
  $font = New-Object System.Drawing.Font 'Segoe UI', 11, ([System.Drawing.FontStyle]::Bold), ([System.Drawing.GraphicsUnit]::Pixel)
  $g.DrawString("$n", $font, [System.Drawing.Brushes]::White, 6, 8)
  $g.Dispose()
  $icon = [System.Drawing.Icon]::FromHandle($bmp.GetHicon())
  return $icon
}

function Stamp([string]$what) {
  if ($Log) {
    $ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    Add-Content -Path $Log -Value "$ms $what"
  }
}

$icons = @()
for ($i = 1; $i -le $Count; $i++) {
  $ni = New-Object System.Windows.Forms.NotifyIcon
  $ni.Icon = New-Picture $i $false
  $ni.Text = "Barista test $i"
  $ni.Visible = $true
  $icons += $ni
}
"added $Count tray icons (pid $PID)"
Stamp "added $Count"

$counter = 0
$flip = $false
if ($Blink -or $Tip) {
  $timer = New-Object System.Windows.Forms.Timer
  $timer.Interval = 2000
  $timer.Add_Tick({
    $script:counter++
    if ($Blink) {
      $script:flip = -not $script:flip
      $icons[0].Icon = New-Picture 1 $script:flip
      Stamp "icon $script:flip"
    }
    if ($Tip) {
      $icons[0].Text = "Barista test 1 #$script:counter"
      Stamp "tip $script:counter"
    }
  })
  $timer.Start()
}

if ($Seconds -gt 0) {
  $quit = New-Object System.Windows.Forms.Timer
  $quit.Interval = $Seconds * 1000
  $quit.Add_Tick({ [System.Windows.Forms.Application]::ExitThread() })
  $quit.Start()
}

try { [System.Windows.Forms.Application]::Run() }
finally {
  foreach ($ni in $icons) { $ni.Visible = $false; $ni.Dispose() }
  Stamp 'removed'
  'removed the tray icons'
}
