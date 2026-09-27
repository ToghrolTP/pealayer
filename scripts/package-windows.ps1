[CmdletBinding()]
param(
    [switch]$NoUpx,
    [switch]$SkipTests,
    [switch]$Run
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if (-not $IsWindows) {
    throw 'scripts/package-windows.ps1 is intended for native Windows packaging.'
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$machineRustupHome = [Environment]::GetEnvironmentVariable('RUSTUP_HOME', 'Machine')
if ($machineRustupHome) { $env:RUSTUP_HOME = $machineRustupHome }
$systemRustBin = Join-Path $env:ProgramFiles 'Rust\bin'
if (Test-Path -LiteralPath (Join-Path $systemRustBin 'cargo.exe')) {
    $env:Path = $systemRustBin + ';' + $env:Path
}
$releaseDirectory = Join-Path $repositoryRoot 'target\release'
$stagingDirectory = Join-Path $repositoryRoot 'target\package-windows'
$sourceDirectory = Split-Path -Parent $repositoryRoot
$outputDirectory = if ((Split-Path -Leaf $sourceDirectory) -ieq 'source') {
    Join-Path (Split-Path -Parent $sourceDirectory) 'bin'
} else {
    Join-Path $repositoryRoot 'bin'
}
$libmpvDirectory = if ($env:LIBMPV_DIR) { $env:LIBMPV_DIR } else { Join-Path $env:ProgramFiles 'MPV' }
$env:Path = $libmpvDirectory + ';' + $env:Path
$upx = Get-Command upx.exe -ErrorAction SilentlyContinue
if (-not $upx) {
    $programFilesUpx = Join-Path $env:ProgramFiles 'UPX\upx.exe'
    if (Test-Path -LiteralPath $programFilesUpx -PathType Leaf) {
        $upx = Get-Item -LiteralPath $programFilesUpx
    }
}

if (-not $SkipTests) {
    $env:LIBMPV_DIR = $libmpvDirectory
    $separator = [char]0x1f
    $linkFlag = "-Lnative=$libmpvDirectory"
    if ($env:CARGO_ENCODED_RUSTFLAGS) {
        $env:CARGO_ENCODED_RUSTFLAGS += $separator + $linkFlag
    } else {
        $env:CARGO_ENCODED_RUSTFLAGS = $linkFlag
    }
    & cargo test --locked --jobs 1
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed with exit code $LASTEXITCODE" }
}

& (Join-Path $PSScriptRoot 'run-windows.ps1') -BuildOnly

New-Item -ItemType Directory -Force -Path $stagingDirectory,$outputDirectory | Out-Null
$stagedExecutable = Join-Path $stagingDirectory 'pealayer.exe'
$stagedRuntime = Join-Path $stagingDirectory 'libmpv-2.dll'
Copy-Item -LiteralPath (Join-Path $releaseDirectory 'pealayer.exe') -Destination $stagedExecutable -Force
Copy-Item -LiteralPath (Join-Path $libmpvDirectory 'libmpv-2.dll') -Destination $stagedRuntime -Force

$resource = (Get-Item -LiteralPath $stagedExecutable).VersionInfo
if ($resource.ProductName -ne 'Pealayer' -or $resource.OriginalFilename -ne 'pealayer.exe') {
    throw 'Packaged executable is missing the expected Win32 identity resources.'
}

$unpackedBytes = (Get-Item -LiteralPath $stagedExecutable).Length
$upxVersion = $null
if (-not $NoUpx) {
    if (-not $upx) { throw 'UPX is required for Windows packaging; install it system-wide or pass -NoUpx explicitly.' }
    $upxVersion = (& $upx.FullName --version | Select-Object -First 1)
    & $upx.FullName --best --lzma $stagedExecutable
    if ($LASTEXITCODE -ne 0) { throw "UPX compression failed with exit code $LASTEXITCODE" }
    & $upx.FullName -t $stagedExecutable
    if ($LASTEXITCODE -ne 0) { throw "UPX validation failed with exit code $LASTEXITCODE" }
}

$env:Path = $stagingDirectory + ';' + $libmpvDirectory + ';' + $env:Path
$smoke = Start-Process -FilePath $stagedExecutable -ArgumentList '--smoke-test' -WorkingDirectory $stagingDirectory -Wait -PassThru
if ($smoke.ExitCode -ne 0) { throw "Packaged Pealayer/libmpv smoke test failed with exit code $($smoke.ExitCode)" }

Copy-Item -LiteralPath $stagedExecutable -Destination $outputDirectory -Force
Copy-Item -LiteralPath $stagedRuntime -Destination $outputDirectory -Force
$webDistribution = Join-Path $repositoryRoot 'web_ui\dist'
$webUiPackaged = $false
if (Test-Path -LiteralPath (Join-Path $webDistribution 'index.html')) {
    $packagedWebDistribution = Join-Path $outputDirectory 'web_ui\dist'
    New-Item -ItemType Directory -Force -Path $packagedWebDistribution | Out-Null
    Copy-Item -Path (Join-Path $webDistribution '*') -Destination $packagedWebDistribution -Recurse -Force
    $webUiPackaged = $true
}

$artifacts = @('pealayer.exe','libmpv-2.dll') | ForEach-Object {
    $path = Join-Path $outputDirectory $_
    [ordered]@{
        path = $_
        bytes = (Get-Item -LiteralPath $path).Length
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
    }
}
$manifest = [ordered]@{
    format = 'pealayer-windows-package/v1'
    version = $resource.ProductVersion
    git_commit = (& git -C $repositoryRoot rev-parse HEAD).Trim()
    git_dirty = [bool](& git -C $repositoryRoot status --porcelain)
    built_at_utc = [DateTime]::UtcNow.ToString('o')
    target = (& rustc -vV | Select-String '^host:' | ForEach-Object { $_.Line.Substring(5).Trim() })
    validation = [ordered]@{
        tests = if ($SkipTests) { 'skipped' } else { 'passed' }
        windows_resources = 'verified'
        libmpv_smoke = 'passed'
        web_ui = if ($webUiPackaged) { 'packaged' } else { 'embedded_fallback' }
        upx = if ($NoUpx) { [ordered]@{ enabled = $false; tested = $false } } else { [ordered]@{ enabled = $true; tested = $true; version = $upxVersion } }
    }
    unpacked_executable_bytes = $unpackedBytes
    artifacts = $artifacts
}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $outputDirectory 'host-manifest.json') -Encoding utf8

Write-Host "Pealayer package published to $outputDirectory"
if ($Run) {
    Start-Process -FilePath (Join-Path $outputDirectory 'pealayer.exe') -WorkingDirectory $outputDirectory
}
