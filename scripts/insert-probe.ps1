# Measure text-insertion success with the production Windows inserter, against a harness-owned
# WinForms text box (never a real app/document). Checks the inserted text matches exactly and that
# the user's clipboard text is unchanged afterwards.
#   powershell -ExecutionPolicy Bypass -File scripts\insert-probe.ps1 -Exe <path to insert_once.exe> [-Rounds 5]
param([Parameter(Mandatory = $true)][string]$Exe, [int]$Rounds = 5)
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class Fg { [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra); }
"@
$title = "Voxelle insertion probe"
$cases = @(
  @{ name = "short ascii (typed)"; text = "Please schedule the design review for Thursday afternoon." },
  # Built from code points so this file stays ASCII (PowerShell 5.1 misreads BOM-less UTF-8):
  # "Cafe' de'ja` vu -- nai"ve, Tokyo in kanji, microphone emoji (surrogate pair)."
  @{ name = "unicode + emoji (typed)"; text = ("Caf{0} d{0}j{1} vu {2} na{3}ve, {4}{5}, emoji {6} ok." -f [char]0xE9, [char]0xE0, [char]0x2014, [char]0xEF, [char]0x6771, [char]0x4EAC, [char]::ConvertFromUtf32(0x1F399)) },
  @{ name = "long text (paste)"; text = ("The quick brown fox jumps over the lazy dog. " * 22).Trim() },
  @{ name = "multi-line (paste)"; text = "first line`nsecond line`nthird line" }
)
$clipBefore = Get-Clipboard -Raw -ErrorAction SilentlyContinue
$results = @()
foreach ($case in $cases) {
  for ($i = 1; $i -le $Rounds; $i++) {
    $form = New-Object System.Windows.Forms.Form
    $form.Text = $title; $form.Width = 700; $form.Height = 260; $form.StartPosition = "CenterScreen"; $form.TopMost = $true
    $box = New-Object System.Windows.Forms.TextBox
    $box.Multiline = $true; $box.Dock = "Fill"; $box.AcceptsReturn = $true
    $form.Controls.Add($box); $form.Show(); $form.Activate(); $box.Focus() | Out-Null
    # Windows only lets a process take the foreground under some conditions; a no-op Alt tap helps.
    # Retry until the probe window is really in front; the Rust side refuses otherwise.
    $wsh = New-Object -ComObject WScript.Shell
    for ($try = 0; $try -lt 15 -and [Fg]::GetForegroundWindow() -ne $form.Handle; $try++) {
      [Fg]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero); [Fg]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
      [Fg]::SetForegroundWindow($form.Handle) | Out-Null
      $wsh.AppActivate($title) | Out-Null
      for ($k = 0; $k -lt 8; $k++) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 25 }
    }
    $box.Focus() | Out-Null
    for ($k = 0; $k -lt 8; $k++) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 25 }
    $tf = [System.IO.Path]::GetTempFileName(); [System.IO.File]::WriteAllText($tf, $case.text, (New-Object System.Text.UTF8Encoding($false)))
    $psi = New-Object System.Diagnostics.ProcessStartInfo $Exe, "--expect-title `"$title`" --text-file `"$tf`""
    $psi.UseShellExecute = $false; $psi.RedirectStandardOutput = $true; $psi.CreateNoWindow = $true
    $p = [System.Diagnostics.Process]::Start($psi)
    while (-not $p.HasExited) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 10 }
    for ($k = 0; $k -lt 40; $k++) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 10 }
    $out = $p.StandardOutput.ReadToEnd() -split "`n" | Where-Object { $_ -like "RESULT *" } | Select-Object -Last 1
    $r = if ($out) { $out.Substring(7) | ConvertFrom-Json } else { [pscustomobject]@{ outcome = "crash" } }
    $expected = ($case.text -replace "`r`n", "`n")
    $got = ($box.Text -replace "`r`n", "`n")
    $match = ($got -eq $expected)
    $form.Close(); $form.Dispose(); Remove-Item $tf
    $results += [pscustomobject]@{ case = $case.name; round = $i; outcome = $r.outcome; method = $r.method; ms = $r.ms; exact = $match; got = if ($match) { "" } else { $got.Substring(0, [Math]::Min(60, $got.Length)) } }
  }
}
$clipAfter = Get-Clipboard -Raw -ErrorAction SilentlyContinue
$results | Format-Table -AutoSize | Out-String -Width 200
$ok = ($results | Where-Object { $_.outcome -eq "Inserted" -and $_.exact }).Count
"insertion success: $ok / $($results.Count) exact matches"
"clipboard text unchanged: $($clipBefore -eq $clipAfter)"
