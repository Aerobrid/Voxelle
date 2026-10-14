# Run the ASR benchmark matrix (model x backend) and record speed, accuracy, peak RAM and VRAM.
#   powershell -ExecutionPolicy Bypass -File scripts\bench-asr.ps1 [-Models a.bin,b.bin] [-Threads 6]
# Build both binaries first (from src-tauri\, in cmd):
#   set CARGO_TARGET_DIR=%USERPROFILE%\.voxelle-cpu&& ..\scripts\win-env.cmd cargo build --release --example asr_bench
#   set CARGO_TARGET_DIR=%USERPROFILE%\.voxelle-vk&& ..\scripts\win-env.cmd cargo build --release --example asr_bench --features gpu-vulkan
# Close heavy apps first: results are only comparable on an idle machine.
param(
  [string[]]$Models = @("ggml-base-q5_1.bin", "ggml-small-q5_1.bin", "ggml-medium-q5_0.bin", "ggml-large-v3-turbo-q5_0.bin"),
  [string[]]$Backends = @("cpu", "vulkan"),
  [string]$ModelDir = (Join-Path $env:LOCALAPPDATA "com.voxelle.app\models"),
  [int]$Threads = 0,
  [string]$Out = (Join-Path $PSScriptRoot "..\bench\results\asr-latest.jsonl")
)
$root = Split-Path $PSScriptRoot -Parent
$corpus = Join-Path $root "bench\corpus"
$targets = @{
  cpu    = if ($env:VOXELLE_CPU_TARGET) { $env:VOXELLE_CPU_TARGET } else { Join-Path $env:USERPROFILE ".voxelle-cpu" }
  vulkan = if ($env:VOXELLE_VK_TARGET) { $env:VOXELLE_VK_TARGET } else { Join-Path $env:USERPROFILE ".voxelle-vk" }
}
New-Item -ItemType Directory -Force (Split-Path $Out) | Out-Null
$hasSmi = [bool](Get-Command nvidia-smi -ErrorAction SilentlyContinue)

function VramUsedMiB { if ($hasSmi) { [int](nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits | Select-Object -First 1) } else { 0 } }

foreach ($m in $Models) {
  foreach ($backend in $Backends) {
    $exe = Join-Path $targets[$backend] "release\examples\asr_bench.exe"
    if (-not (Test-Path $exe)) { Write-Warning "skip $backend (no binary at $exe)"; continue }
    $argv = @("--model", ("`"" + (Join-Path $ModelDir $m) + "`""), "--corpus", ("`"" + $corpus + "`""))
    if ($backend -eq "vulkan") { $argv += "--gpu" }
    if ($Threads -gt 0) { $argv += @("--threads", "$Threads") }
    $baseVram = VramUsedMiB
    $stdout = [System.IO.Path]::GetTempFileName()
    $p = Start-Process -FilePath $exe -ArgumentList $argv -NoNewWindow -PassThru -RedirectStandardOutput $stdout -RedirectStandardError "$stdout.err"
    $peakVram = $baseVram; $peakRam = 0
    while (-not $p.HasExited) {
      $v = VramUsedMiB; if ($v -gt $peakVram) { $peakVram = $v }
      try { $p.Refresh(); if ($p.PeakWorkingSet64 -gt $peakRam) { $peakRam = $p.PeakWorkingSet64 } } catch {}
      Start-Sleep -Milliseconds 250
    }
    $p.WaitForExit()
    $peakRamMiB = [math]::Round($peakRam / 1MB)
    $line = Get-Content $stdout | Where-Object { $_ -like "RESULT *" } | Select-Object -Last 1
    if (-not $line) { Write-Warning "$m/$backend failed:"; Get-Content "$stdout.err" -Tail 5; continue }
    $r = $line.Substring(7) | ConvertFrom-Json
    $r | Add-Member backend $backend
    $r | Add-Member peakRamMiB $peakRamMiB
    $r | Add-Member vramDeltaMiB ($peakVram - $baseVram)
    $r | Add-Member date (Get-Date -Format s)
    Add-Content -Path $Out -Value ($r | ConvertTo-Json -Compress) -Encoding utf8
    "{0,-30} {1,-6} gpu={2,-5} load={3,5}ms first={4,6}ms mean={5,6}ms p50={6,6}ms rtf={7,6} wer={8,6} ram={9,5}MiB vram+={10,5}MiB" -f $m, $backend, $r.gpuActive, $r.loadMs, $r.firstCallMs, $r.meanMs, $r.p50Ms, $r.rtf, $r.wer, $peakRamMiB, ($peakVram - $baseVram)
    Remove-Item $stdout, "$stdout.err" -ErrorAction SilentlyContinue
  }
}
