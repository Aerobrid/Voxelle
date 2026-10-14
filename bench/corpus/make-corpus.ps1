# Render bench/corpus/sentences.tsv to 16 kHz mono WAVs using the offline Windows SAPI voice.
param([string]$OutDir = (Join-Path $PSScriptRoot "audio"))
Add-Type -AssemblyName System.Speech
New-Item -ItemType Directory -Force $OutDir | Out-Null
$format = New-Object System.Speech.AudioFormat.SpeechAudioFormatInfo(16000, [System.Speech.AudioFormat.AudioBitsPerSample]::Sixteen, [System.Speech.AudioFormat.AudioChannel]::Mono)
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
$synth.Rate = 0
$rows = Import-Csv (Join-Path $PSScriptRoot "sentences.tsv") -Delimiter "`t"
foreach ($row in $rows) {
  $path = Join-Path $OutDir "$($row.id).wav"
  $synth.SetOutputToWaveFile($path, $format)
  $synth.Speak($row.text)
}
$synth.SetOutputToNull()
$synth.Dispose()
"voice: $($synth.Voice.Name)"
Get-ChildItem $OutDir -Filter *.wav | Select-Object Name, Length
