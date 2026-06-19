param(
  [string]$Output = ".\test-media",
  [switch]$IncludeMpvPlaceholders
)

$ErrorActionPreference = "Stop"
$rootItem = New-Item -ItemType Directory -Force -Path $Output
$root = $rootItem.FullName

Get-ChildItem -LiteralPath $root -Force | Remove-Item -Recurse -Force

function Write-Bytes([string]$Path, [byte[]]$Bytes) {
  [System.IO.File]::WriteAllBytes($Path, $Bytes)
}

function New-Wav([string]$Path, [int]$Seconds = 3) {
  $sampleRate = 44100
  $channels = 1
  $bitsPerSample = 16
  $byteRate = [int]($sampleRate * $channels * ($bitsPerSample / 8))
  $blockAlign = [int]($channels * ($bitsPerSample / 8))
  $dataSize = [int]($sampleRate * $channels * ($bitsPerSample / 8) * $Seconds)
  $stream = [System.IO.MemoryStream]::new()
  $writer = [System.IO.BinaryWriter]::new($stream)
  $writer.Write([Text.Encoding]::ASCII.GetBytes("RIFF"))
  $writer.Write([int](36 + $dataSize))
  $writer.Write([Text.Encoding]::ASCII.GetBytes("WAVEfmt "))
  $writer.Write([int]16)
  $writer.Write([int16]1)
  $writer.Write([int16]$channels)
  $writer.Write([int]$sampleRate)
  $writer.Write([int]$byteRate)
  $writer.Write([int16]$blockAlign)
  $writer.Write([int16]$bitsPerSample)
  $writer.Write([Text.Encoding]::ASCII.GetBytes("data"))
  $writer.Write([int]$dataSize)
  $writer.Write([byte[]]::new($dataSize))
  $writer.Dispose()
  Write-Bytes $Path $stream.ToArray()
}

function Invoke-Ffmpeg([string[]]$Arguments) {
  & ffmpeg @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "ffmpeg failed: ffmpeg $($Arguments -join ' ')"
  }
}

$ffmpeg = Get-Command ffmpeg -ErrorAction SilentlyContinue

New-Wav (Join-Path $root "playable-wav-3s.wav") 3

if ($ffmpeg) {
  Invoke-Ffmpeg @(
    "-y",
    "-f", "lavfi",
    "-i", "sine=frequency=440:duration=3",
    "-c:a", "libmp3lame",
    (Join-Path $root "playable-mp3-3s.mp3")
  )

  Invoke-Ffmpeg @(
    "-y",
    "-f", "lavfi",
    "-i", "testsrc=size=640x360:rate=25:duration=5",
    "-f", "lavfi",
    "-i", "sine=frequency=440:duration=5",
    "-c:v", "libx264",
    "-pix_fmt", "yuv420p",
    "-c:a", "aac",
    "-shortest",
    (Join-Path $root "playable-mp4-5s.mp4")
  )

  Invoke-Ffmpeg @(
    "-y",
    "-f", "lavfi",
    "-i", "testsrc=size=640x360:rate=25:duration=5",
    "-f", "lavfi",
    "-i", "sine=frequency=440:duration=5",
    "-c:v", "libx264",
    "-pix_fmt", "yuv420p",
    "-c:a", "aac",
    "-shortest",
    "-f", "mpegts",
    (Join-Path $root "playable-ts-5s.ts")
  )

  Invoke-Ffmpeg @(
    "-y",
    "-i", (Join-Path $root "playable-mp4-5s.mp4"),
    "-c", "copy",
    "-hls_time", "2",
    "-hls_list_size", "0",
    "-hls_segment_filename", (Join-Path $root "hls-segment-%03d.ts"),
    (Join-Path $root "playable-hls.m3u8")
  )

  Set-Content -LiteralPath (Join-Path $root "playable-mp4-5s.srt") -Encoding utf8 -Value @(
    "1"
    "00:00:00,000 --> 00:00:02,000"
    "Open Course Player subtitle test"
  )
  Set-Content -LiteralPath (Join-Path $root "playable-ts-5s.srt") -Encoding utf8 -Value @(
    "1"
    "00:00:00,000 --> 00:00:02,000"
    "MPEG-TS subtitle test"
  )
} else {
  Set-Content -LiteralPath (Join-Path $root "README.txt") -Encoding utf8 -Value @(
    "ffmpeg was not found in PATH."
    "Only one real playable WAV file was generated: playable-wav-3s.wav"
    ""
    "To generate real playable MP4, TS, HLS and MP3 fixtures, install ffmpeg, add it to PATH, and rerun:"
    "powershell -ExecutionPolicy Bypass -File tools/media-fixtures/generate-fixtures.ps1 -Output test-media"
    ""
    "By default this script no longer creates non-playable placeholder files like sample.mkv or sample.vob."
  )
}

if ($IncludeMpvPlaceholders) {
  $mpvDir = New-Item -ItemType Directory -Force -Path (Join-Path $root "mpv-placeholder-formats")
  Set-Content -LiteralPath (Join-Path $mpvDir.FullName "README.txt") -Encoding utf8 -Value "These files are placeholders for scan classification only. They are not playable media."
  foreach ($name in "sample.mkv","sample.avi","sample.flv","sample.mov","sample.wmv","sample.rmvb","sample.vob","sample.3gp","sample.mpeg","sample.mpg") {
    Write-Bytes (Join-Path $mpvDir.FullName $name) ([byte[]](0x00,0x00,0x00,0x00))
  }
}

Write-Host "Generated media fixture directory: $root"
