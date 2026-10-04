[CmdletBinding()]
param(
    [switch]$DebugBuild,
    [switch]$BuildOnly,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ApplicationArguments
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'scripts/run-windows.ps1 is intended for native Windows builds.'
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot
$cargoTargetDirectory = if ($env:CARGO_TARGET_DIR) {
    if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
        [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
    } else {
        [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $env:CARGO_TARGET_DIR))
    }
} else {
    Join-Path $repositoryRoot 'target'
}
$machineRustupHome = [Environment]::GetEnvironmentVariable('RUSTUP_HOME', 'Machine')
if ($machineRustupHome) { $env:RUSTUP_HOME = $machineRustupHome }
$userProfileDirectory = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
$rustBin = @(
    (Join-Path $env:ProgramFiles 'Rust\bin')
    (Join-Path $userProfileDirectory '.cargo\bin')
) + @(
    Get-ChildItem -LiteralPath (Join-Path $userProfileDirectory '.rustup\toolchains') -Directory -ErrorAction SilentlyContinue |
        Sort-Object -Property Name |
        ForEach-Object { Join-Path $_.FullName 'bin' }
) | Where-Object {
    $cargo = Get-Item -LiteralPath (Join-Path $_ 'cargo.exe') -ErrorAction SilentlyContinue
    $rustc = Get-Item -LiteralPath (Join-Path $_ 'rustc.exe') -ErrorAction SilentlyContinue
    $cargo -and $cargo.Length -gt 0 -and $rustc -and $rustc.Length -gt 0
} | Select-Object -First 1
if ($rustBin) {
    $env:Path = $rustBin + ';' + $env:Path
}

$libmpvSourceDirectory = if ($env:LIBMPV_DIR) {
    $env:LIBMPV_DIR
} else {
    Join-Path $env:ProgramFiles 'MPV'
}
$libmpvDirectory = $libmpvSourceDirectory

$rustHost = (& rustc -vV | Select-String '^host:' | ForEach-Object { $_.Line.Substring(5).Trim() })
if (-not $rustHost) { throw 'Could not determine the native Rust host triple.' }
$importLibraryNames = if ($rustHost -like '*-msvc') {
    @('mpv.lib')
} else {
    @('libmpv.dll.a', 'libmpv.a')
}
$libmpvImportLibrary = $importLibraryNames |
    ForEach-Object { Join-Path $libmpvSourceDirectory $_ } |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
    Select-Object -First 1
if (-not $libmpvImportLibrary -and $rustHost -like '*-msvc') {
    $gnuImportLibrary = Join-Path $libmpvSourceDirectory 'libmpv.dll.a'
    if (Test-Path -LiteralPath $gnuImportLibrary -PathType Leaf) {
        $libmpvDirectory = Join-Path $cargoTargetDirectory 'mpv-msvc-import'
        New-Item -ItemType Directory -Force -Path $libmpvDirectory | Out-Null
        $libmpvImportLibrary = Join-Path $libmpvDirectory 'mpv.lib'
        Copy-Item -LiteralPath $gnuImportLibrary -Destination $libmpvImportLibrary -Force
    }
}
if (-not $libmpvImportLibrary) {
    throw "Required libmpv import library for $rustHost is missing. Expected one of: $($importLibraryNames -join ', ') in $libmpvSourceDirectory"
}
$libmpvRuntime = @('libmpv-2.dll', 'mpv-2.dll') |
    ForEach-Object { Join-Path $libmpvSourceDirectory $_ } |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
    Select-Object -First 1
if (-not $libmpvRuntime) {
    throw "Required libmpv runtime is missing. Expected libmpv-2.dll or mpv-2.dll in $libmpvDirectory"
}
foreach ($requiredPath in @($libmpvImportLibrary, $libmpvRuntime)) {
    if (-not (Test-Path -LiteralPath $requiredPath)) {
        throw "Required libmpv file is missing: $requiredPath"
    }
}

$cargo = Get-Command cargo -ErrorAction Stop
$env:Path = $libmpvDirectory + ';' + $env:Path
$linkFlag = "-Lnative=$libmpvDirectory"
if ($env:CARGO_ENCODED_RUSTFLAGS) {
    $env:CARGO_ENCODED_RUSTFLAGS += [char]0x1f + $linkFlag
} else {
    $env:CARGO_ENCODED_RUSTFLAGS = $linkFlag
}

$cargoArguments = @('build', '--locked')
$profileDirectory = 'debug'
if (-not $DebugBuild) {
    $cargoArguments += '--release'
    $profileDirectory = 'release'
}

& $cargo.Source @cargoArguments
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}

$binaryDirectory = Join-Path $cargoTargetDirectory $profileDirectory
$binary = Join-Path $binaryDirectory 'pealayer.exe'
if (-not (Test-Path -LiteralPath $binary)) {
    throw "Pealayer binary was not produced at $binary"
}
Copy-Item -LiteralPath $libmpvRuntime -Destination (Join-Path $binaryDirectory 'libmpv-2.dll') -Force

if (-not $BuildOnly) {
    & $binary @ApplicationArguments
}
