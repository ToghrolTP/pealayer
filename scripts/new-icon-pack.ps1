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

function Remove-IconTempDirectory([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $resolved = (Resolve-Path -LiteralPath $Path).Path
    $directory = [IO.DirectoryInfo]::new($resolved)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd([char[]]'\/')
    if ($directory.Parent.FullName.TrimEnd([char[]]'\/') -ine $tempRoot -or
        $directory.Name -notmatch '^pealayer-icon-[a-f0-9]{32}$') {
        throw "Refusing to remove an unexpected icon staging directory: $resolved"
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}

function Write-MultiSizeIco([string]$Source, [string]$Destination) {
    $sizes = @(16, 24, 32, 48, 64, 128, 256)
    $temp = Join-Path ([IO.Path]::GetTempPath()) ("pealayer-icon-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $temp | Out-Null
    try {
        $images = [Collections.Generic.List[byte[]]]::new()
        foreach ($size in $sizes) {
            $png = Join-Path $temp "$size.png"
            Invoke-Ffmpeg @('-i', $Source, '-vf', "scale=${size}:${size}:flags=lanczos:force_original_aspect_ratio=decrease,pad=${size}:${size}:(ow-iw)/2:(oh-ih)/2:color=0x00000000", '-frames:v', '1', $png)
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
        Remove-IconTempDirectory $temp
    }
}

$script:FfmpegPath = Resolve-Ffmpeg $Ffmpeg
$output = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $output -Force | Out-Null

$sourceTemp = $null
try {
    if ($PSCmdlet.ParameterSetName -eq 'Pair') {
        $pair = Resolve-ExistingFile $VerticalPair 'Vertical pair'
        $sourceTemp = Join-Path ([IO.Path]::GetTempPath()) ("pealayer-icon-" + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $sourceTemp | Out-Null
        $playingSource = Join-Path $sourceTemp 'playing.png'
        $stoppedSource = Join-Path $sourceTemp 'stopped.png'
        Invoke-Ffmpeg @('-i', $pair, '-vf', 'crop=iw/2:ih:0:0', '-frames:v', '1', $playingSource)
        Invoke-Ffmpeg @('-i', $pair, '-vf', 'crop=iw/2:ih:iw/2:0', '-frames:v', '1', $stoppedSource)
        $pausedSource = $stoppedSource
    } else {
        $playingSource = Resolve-ExistingFile $Playing 'Playing icon'
        $stoppedSource = Resolve-ExistingFile $Stopped 'Stopped icon'
        $pausedSource = if ($Paused) { Resolve-ExistingFile $Paused 'Paused icon' } else { $stoppedSource }
    }
    foreach ($state in @(
        @{Name='playing'; Source=$playingSource},
        @{Name='paused'; Source=$pausedSource},
        @{Name='stopped'; Source=$stoppedSource}
    )) {
        Write-MultiSizeIco $state.Source (Join-Path $output ($state.Name + '.ico'))
    }
    Copy-Item -LiteralPath (Join-Path $output 'stopped.ico') -Destination (Join-Path $output 'app.ico') -Force
} finally {
    if ($sourceTemp) { Remove-IconTempDirectory $sourceTemp }
}

@{
    format = 'pealayer-icon-pack'
    playing = 'playing.ico'
    paused = 'paused.ico'
    stopped = 'stopped.ico'
    windows = 'app.ico'
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $output 'icon-pack.json') -Encoding utf8

Write-Output "Created Pealayer icon pack: $output"
