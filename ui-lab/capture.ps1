param([string]$OutDir = "ui-lab-results")
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class U32 {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
"@

function Capture-App($exe, $name) {
  $out = Join-Path $OutDir $name
  New-Item -ItemType Directory -Force -Path $out | Out-Null
  $env:UI_LAB_DEMO = "1"
  $env:UI_LAB_METRICS = (Join-Path $out "app-metrics.json")
  $p = Start-Process $exe -PassThru
  $t0 = Get-Date
  $h = [IntPtr]::Zero
  while ($h -eq [IntPtr]::Zero -and ((Get-Date) - $t0).TotalSeconds -lt 60) {
    Start-Sleep -Milliseconds 100
    $p.Refresh()
    $h = $p.MainWindowHandle
  }
  $readyMs = [int]((Get-Date) - $t0).TotalMilliseconds
  $frames = 0; $memMax = 0L
  $t1 = Get-Date
  while (((Get-Date) - $t1).TotalSeconds -lt 13) {
    if ($h -ne [IntPtr]::Zero -and -not $p.HasExited) {
      $r = New-Object U32+RECT
      [U32]::GetWindowRect($h, [ref]$r) | Out-Null
      $w = $r.Right - $r.Left; $hh = $r.Bottom - $r.Top
      if ($w -gt 0 -and $hh -gt 0) {
        try {
          $bmp = New-Object System.Drawing.Bitmap $w, $hh
          $g = [System.Drawing.Graphics]::FromImage($bmp)
          $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
          $bmp.Save((Join-Path $out ("frame_{0:d3}.png" -f $frames)))
          $g.Dispose(); $bmp.Dispose()
          $frames++
        } catch { }
      }
    }
    $p.Refresh()
    if (-not $p.HasExited) { $memMax = [Math]::Max($memMax, $p.WorkingSet64) }
    Start-Sleep -Milliseconds 125
  }
  $exited = $p.HasExited
  if (-not $exited) { $p.Kill() }
  Remove-Item Env:UI_LAB_DEMO -ErrorAction SilentlyContinue
  Remove-Item Env:UI_LAB_METRICS -ErrorAction SilentlyContinue
  @{ app=$name; window_ready_ms=$readyMs; frames=$frames; mem_mb=[math]::Round($memMax/1MB,1); exited_clean=$exited } |
    ConvertTo-Json | Out-File (Join-Path $out "metrics.json")
}

Capture-App "ui-lab\slint-demo\target\release\ui-lab-slint.exe" "slint"
Capture-App "ui-lab\tauri-demo\src-tauri\target\release\ui-lab-tauri.exe" "tauri"
