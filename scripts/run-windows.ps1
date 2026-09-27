[CmdletBinding()]
param(
    [switch]$DebugBuild,
    [switch]$BuildOnly,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ApplicationArguments
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $IsWindows) {
    throw 'scripts/run-windows.ps1 is intended for native Windows builds.'
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repositoryRoot
$machineRustupHome = [Environment]::GetEnvironmentVariable('RUSTUP_HOME', 'Machine')
if ($machineRustupHome) { $env:RUSTUP_HOME = $machineRustupHome }
$systemRustBin = Join-Path $env:ProgramFiles 'Rust\bin'
if (Test-Path -LiteralPath (Join-Path $systemRustBin 'cargo.exe')) {
    $env:Path = $systemRustBin + ';' + $env:Path
}

$libmpvDirectory = if ($env:LIBMPV_DIR) {
    $env:LIBMPV_DIR
} else {
    Join-Path $env:ProgramFiles 'MPV'
}

$libmpvImportLibrary = Join-Path $libmpvDirectory 'libmpv.dll.a'
$libmpvRuntime = Join-Path $libmpvDirectory 'libmpv-2.dll'
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

$binaryDirectory = Join-Path $repositoryRoot "target\$profileDirectory"
$binary = Join-Path $binaryDirectory 'pealayer.exe'
if (-not (Test-Path -LiteralPath $binary)) {
    throw "Pealayer binary was not produced at $binary"
}
Copy-Item -LiteralPath $libmpvRuntime -Destination $binaryDirectory -Force

if (-not $BuildOnly) {
    & $binary @ApplicationArguments
}
