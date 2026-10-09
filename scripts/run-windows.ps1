[CmdletBinding()]
param(
    [switch]$DebugBuild,
    [switch]$BuildOnly,
    [string]$LibmpvDirectory,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ApplicationArguments
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'scripts/run-windows.ps1 is intended for native Windows builds.'
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'libmpv-windows.ps1')
Set-Location -LiteralPath $repositoryRoot
$cargoTargetDirectory = Get-PealayerCargoTargetDirectory -RepositoryRoot $repositoryRoot
$env:CARGO_TARGET_DIR = $cargoTargetDirectory
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

$rustHost = (& rustc -vV | Select-String '^host:' | ForEach-Object { $_.Line.Substring(5).Trim() })
if (-not $rustHost) { throw 'Could not determine the native Rust host triple.' }
$libmpv = Resolve-PealayerLibmpv -RepositoryRoot $repositoryRoot -RustHost $rustHost -ExplicitDirectory $LibmpvDirectory
$libmpvSourceDirectory = $libmpv.SourceDirectory
$libmpvDirectory = $libmpv.LinkDirectory
$libmpvImportLibrary = $libmpv.ImportLibrary
$libmpvRuntime = $libmpv.RuntimeLibrary
Save-PealayerWindowsHostProfile -RepositoryRoot $repositoryRoot -Resolution $libmpv -PersistUserEnvironment | Out-Null
Set-PealayerLibmpvBuildEnvironment -Resolution $libmpv

$cargo = Get-Command cargo -ErrorAction Stop
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
Copy-PealayerLibmpvRuntime -RuntimeLibrary $libmpvRuntime -DestinationDirectory $binaryDirectory

if (-not $BuildOnly) {
    & $binary @ApplicationArguments
}
