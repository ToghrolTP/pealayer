[CmdletBinding(DefaultParameterSetName = 'Separate')]
param(
    [Parameter(Mandatory, ParameterSetName = 'Pair')]
    [string]$VerticalPair,
    [Parameter(Mandatory, ParameterSetName = 'Separate')]
    [string]$Playing,
    [Parameter(ParameterSetName = 'Separate')]
    [string]$Paused,
    [Parameter(Mandatory, ParameterSetName = 'Separate')]
    [string]$Stopped,
    [Parameter(Mandatory)]
    [string]$OutputDirectory,
    [string]$Ffmpeg
)

$ErrorActionPreference = 'Stop'

function Resolve-ExistingFile([string]$Path, [string]$Label) {
    $resolved = Resolve-Path -LiteralPath $Path -ErrorAction Stop
    if (-not (Test-Path -LiteralPath $resolved.Path -PathType Leaf)) {
        throw "$Label is not a file: $Path"
    }
    $resolved.Path
}

function Resolve-Ffmpeg([string]$Requested) {
    if ($Requested) { return Resolve-ExistingFile $Requested 'FFmpeg' }
    $command = Get-Command ffmpeg.exe -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $programFilesCandidate = Join-Path $env:ProgramFiles 'MPV\ffmpeg.exe'
    if (Test-Path -LiteralPath $programFilesCandidate) { return $programFilesCandidate }
    throw 'FFmpeg was not found. Install it system-wide or pass -Ffmpeg.'
}

function Invoke-Ffmpeg([string[]]$Arguments) {
    & $script:FfmpegPath -hide_banner -loglevel error -y @Arguments
    if ($LASTEXITCODE -ne 0) { throw "FFmpeg failed with exit code $LASTEXITCODE" }
}

function Write-MultiSizeIco([string]$Source, [string]$Destination) {
    $sizes = @(16, 24, 32, 48, 64, 128, 256)
    $temp = Join-Path ([IO.Path]::GetTempPath()) ("pealayer-icon-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $temp | Out-Null
    try {
        $images = [Collections.Generic.List[byte[]]]::new()
        foreach ($size in $sizes) {
            $png = Join-Path $temp "$size.png"
            Invoke-Ffmpeg @('-i', $Source, '-vf', "scale=${size}:${size}:force_original_aspect_ratio=decrease,pad=${size}:${size}:(ow-iw)/2:(oh-ih)/2:color=0x00000000", '-frames:v', '1', $png)
            $images.Add([IO.File]::ReadAllBytes($png))
        }
        $stream = [IO.MemoryStream]::new()
        $writer = [IO.BinaryWriter]::new($stream)
        try {
            $writer.Write([UInt16]0)
            $writer.Write([UInt16]1)
            $writer.Write([UInt16]$images.Count)
            $offset = 6 + (16 * $images.Count)
            for ($index = 0; $index -lt $images.Count; $index++) {
                $size = $sizes[$index]
                $writer.Write([Byte]($(if ($size -eq 256) { 0 } else { $size })))
                $writer.Write([Byte]($(if ($size -eq 256) { 0 } else { $size })))
                $writer.Write([Byte]0)
                $writer.Write([Byte]0)
                $writer.Write([UInt16]1)
                $writer.Write([UInt16]32)
                $writer.Write([UInt32]$images[$index].Length)
                $writer.Write([UInt32]$offset)
                $offset += $images[$index].Length
            }
            foreach ($image in $images) { $writer.Write($image) }
            $writer.Flush()
            [IO.File]::WriteAllBytes($Destination, $stream.ToArray())
        } finally {
            $writer.Dispose()
            $stream.Dispose()
        }
    } finally {
        Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

$script:FfmpegPath = Resolve-Ffmpeg $Ffmpeg
$output = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $output -Force | Out-Null

if ($PSCmdlet.ParameterSetName -eq 'Pair') {
    $pair = Resolve-ExistingFile $VerticalPair 'Vertical pair'
    Invoke-Ffmpeg @('-i', $pair, '-vf', 'crop=iw/2:ih:0:0', '-frames:v', '1', (Join-Path $output 'playing.png'))
    Invoke-Ffmpeg @('-i', $pair, '-vf', 'crop=iw/2:ih:iw/2:0', '-frames:v', '1', (Join-Path $output 'stopped.png'))
    Copy-Item -LiteralPath (Join-Path $output 'stopped.png') -Destination (Join-Path $output 'paused.png') -Force
} else {
    Invoke-Ffmpeg @('-i', (Resolve-ExistingFile $Playing 'Playing icon'), '-frames:v', '1', (Join-Path $output 'playing.png'))
    Invoke-Ffmpeg @('-i', (Resolve-ExistingFile $Stopped 'Stopped icon'), '-frames:v', '1', (Join-Path $output 'stopped.png'))
    if ($Paused) {
        Invoke-Ffmpeg @('-i', (Resolve-ExistingFile $Paused 'Paused icon'), '-frames:v', '1', (Join-Path $output 'paused.png'))
    } else {
        Copy-Item -LiteralPath (Join-Path $output 'stopped.png') -Destination (Join-Path $output 'paused.png') -Force
    }
}

Write-MultiSizeIco (Join-Path $output 'stopped.png') (Join-Path $output 'app.ico')
@{
    format = 'pealayer-icon-pack'
    playing = 'playing.png'
    paused = 'paused.png'
    stopped = 'stopped.png'
    windows = 'app.ico'
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $output 'icon-pack.json') -Encoding utf8

Write-Output "Created Pealayer icon pack: $output"
