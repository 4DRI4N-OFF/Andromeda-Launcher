# Visual proof for the startup first-frame repair (invoked by startup-verify.yml).
#
# What this proves: the app leaves the splash and shows the real main window
# titled "Andromeda Launcher", with non-black pixels sampled INSIDE that
# window's own rectangle. What it does NOT prove (declared limits):
# - The 2s title poll cannot verify the 16ms handoff tick; the saved frame
#   sequence is coarse evidence of the transition, not a tick-level check. Fine
#   transition smoothness stays PENDING real-device validation; we do not claim
#   a perfect transition from this run.
# - On the runner Home is the EMPTY state (throwaway data dir: no instances,
#   no accounts); we do not claim a populated catalog.
# - The app log's warn line is recorded through tracing_appender (reliable);
#   Slint/GL stderr logs are NOT guaranteed and are not asserted.
# Attribution: the workflow preserves and removes the smoke run's logs before
# this script runs, so data/logs here belongs ONLY to this visual execution.
$ErrorActionPreference = "Stop"
$out = "verify-out"
$cap = "$out/caps"; New-Item -ItemType Directory -Force $cap | Out-Null
$seq = "$cap/seq"; New-Item -ItemType Directory -Force $seq | Out-Null
$logs = "$out/logs"; New-Item -ItemType Directory -Force $logs | Out-Null
Add-Type -AssemblyName System.Drawing
$sig = @'
using System;
using System.Text;
using System.Runtime.InteropServices;
using System.Collections.Generic;
public class WinEnum {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
  public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
  public static List<string> TitlesForPid(uint target) {
    var titles = new List<string>();
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == target && IsWindowVisible(h)) {
        var sb = new StringBuilder(512); GetWindowText(h, sb, sb.Capacity);
        titles.Add(sb.ToString());
      }
      return true;
    }, IntPtr.Zero);
    return titles;
  }
  public static IntPtr HwndForTitle(uint target, string wanted) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (pid == target && IsWindowVisible(h)) {
        var sb = new StringBuilder(512); GetWindowText(h, sb, sb.Capacity);
        if (sb.ToString() == wanted) { found = h; return false; }
      }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
'@
Add-Type -TypeDefinition $sig
# Fresh env: no inherited SLINT_BACKEND/SLINT_SCALE_FACTOR; throwaway profile dirs.
Remove-Item Env:SLINT_BACKEND -ErrorAction SilentlyContinue
Remove-Item Env:SLINT_SCALE_FACTOR -ErrorAction SilentlyContinue
$env:HOME = "$env:RUNNER_TEMP/fakehome"; $env:APPDATA = "$env:RUNNER_TEMP/fakeappdata"
$env:RUST_LOG = "info"; $env:SLINT_LOGGING = "info"
$exe = Join-Path $PWD "target/release/andromeda.exe"
$p = Start-Process -FilePath $exe -PassThru `
     -RedirectStandardOutput "$logs/out-visual.log" -RedirectStandardError "$logs/err-visual.log"
# Poll for the real main window. Splash title is "Andromeda"; the main window
# title is "Andromeda Launcher". A failed first-frame gate shows an error
# dialog over the splash, so the title never changes: waiting for the exact
# main title is what distinguishes Home from modal/splash. One desktop frame
# per tick is saved as a coarse transition sequence (see header limits).
$deadline = (Get-Date).AddSeconds(240)
$homeReached = $false
$lastTitles = ""
$tick = 0
while ((Get-Date) -lt $deadline) {
  Start-Sleep -Seconds 2
  $tick++
  if ($p.HasExited) {
    Write-Host "andromeda.exe exited on its own (code $($p.ExitCode)) before reaching Home"
    break
  }
  $p.Refresh()
  $titles = [WinEnum]::TitlesForPid($p.Id)
  $joined = ($titles | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join " | "
  if ($joined -ne $lastTitles) { Write-Host "window titles: $joined"; $lastTitles = $joined }
  if ($titles -contains "Andromeda Launcher") { $homeReached = $true; break }
  if (($tick % 5) -eq 0) {
    $sb = New-Object System.Drawing.Bitmap 1920, 1080
    $sg = [System.Drawing.Graphics]::FromImage($sb)
    $sg.CopyFromScreen(0, 0, 0, 0, $sb.Size)
    $sb.Save(("{0}/seq-{1:D3}.png" -f $seq, $tick))
    $sg.Dispose(); $sb.Dispose()
  }
}
# After Home: 5 frames 1s apart to observe the startup overlay fade settling
# (coarse; the overlay fade is ~hundreds of ms, so this brackets its end).
if ($homeReached) {
  foreach ($i in 1..5) {
    Start-Sleep -Seconds 1
    $hb = New-Object System.Drawing.Bitmap 1920, 1080
    $hg = [System.Drawing.Graphics]::FromImage($hb)
    $hg.CopyFromScreen(0, 0, 0, 0, $hb.Size)
    $hb.Save("$seq/home-settle-$i.png")
    $hg.Dispose(); $hb.Dispose()
  }
}
$titlesNow = [WinEnum]::TitlesForPid($p.Id)
($titlesNow | ForEach-Object { "'$_'" }) -join ", " | Out-File "$logs/windows-final.txt"
# Full desktop frame kept as diagnostic context only; it is NOT the proof.
$db = New-Object System.Drawing.Bitmap 1920, 1080
$dg = [System.Drawing.Graphics]::FromImage($db)
$dg.CopyFromScreen(0, 0, 0, 0, $db.Size)
$db.Save("$cap/desktop-final.png")
$dg.Dispose(); $db.Dispose()
# The proof: capture INSIDE the main window's own rectangle and sample there.
$windowOk = $false
if ($homeReached) {
  $hwnd = [WinEnum]::HwndForTitle($p.Id, "Andromeda Launcher")
  if ($hwnd -ne [IntPtr]::Zero) {
    $r = New-Object WinEnum+RECT
    [WinEnum]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
    if ($w -gt 0 -and $h -gt 0) {
      $wb = New-Object System.Drawing.Bitmap $w, $h
      $wg = [System.Drawing.Graphics]::FromImage($wb)
      $wg.CopyFromScreen($r.Left, $r.Top, 0, 0, $wb.Size)
      $wb.Save("$cap/home-final.png")
      $samples = 0; $lit = 0
      foreach ($fx in 0.1, 0.3, 0.5, 0.7, 0.9) { foreach ($fy in 0.1, 0.3, 0.5, 0.7, 0.9) {
        $samples++
        $px = $wb.GetPixel([int]($w * $fx), [int]($h * $fy))
        if (($px.R + $px.G + $px.B) -gt 45) { $lit++ }
      } }
      $wg.Dispose(); $wb.Dispose()
      Write-Host "window-rect capture ${w}x${h} at ($($r.Left),$($r.Top)): $lit/$samples lit samples"
      if ($lit -gt 0) { $windowOk = $true }
    }
  }
}
Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
# Per-execution app diagnostics. data_dir is compile-time (main.rs data_dir():
# CARGO_MANIFEST_DIR.parent()/data with no .andromeda-test marker), so on the
# runner it is deterministically $GITHUB_WORKSPACE/data/logs. The workflow
# removed the smoke run's logs before this script, so these files are this
# execution's alone.
$appLogDir = Join-Path $env:GITHUB_WORKSPACE "data/logs"
$appLogs = @()
if (Test-Path $appLogDir) { $appLogs = @(Get-ChildItem -Path $appLogDir -Filter "andromeda.log.*" -File -ErrorAction SilentlyContinue) }
if ($appLogs.Count -eq 0) { Write-Error "no $appLogDir/andromeda.log.* produced by the visual run; diagnostics missing"; exit 1 }
New-Item -ItemType Directory -Force "$logs/app" | Out-Null
foreach ($f in $appLogs) {
  if ($f.Length -eq 0) { Write-Error "app log $($f.Name) is empty; diagnostics missing"; exit 1 }
  Copy-Item $f.FullName "$logs/app/" -Force
}
Write-Host "app logs preserved: $($appLogs.Count) file(s) from $appLogDir"
# Which first-frame path did THIS run take? On the runner's software renderer
# the repair degrades explicitly; the warn line in THIS run's log is the proof.
# Absence means the renderer supports the notifier (verified path).
$degraded = Select-String -Path "$logs/app/*" -Pattern "first-frame verification unavailable" -SimpleMatch -Quiet
if ($degraded) { Write-Host "degraded path taken (renderer lacks rendering notifier): explicit unverified-frame continuation" }
else { Write-Host "degraded line absent in this run's log: renderer supports the notifier (verified path); see app logs" }
if (-not $homeReached) { Write-Error "main window 'Andromeda Launcher' never appeared within 240s; final titles in windows-final.txt, sequence in caps/seq/, desktop-final.png"; exit 1 }
if (-not $windowOk) { Write-Error "main window title seen but its rectangle capture was black or unavailable; desktop-final.png kept as diagnostic only"; exit 1 }
Write-Host "HOME REACHED: main window 'Andromeda Launcher' rendered (non-black pixels inside its own rect; empty state, catalog population not claimed)"
