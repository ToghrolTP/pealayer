[CmdletBinding()]
param(
    [switch]$NoUpx,
    [switch]$SkipTests,
    [switch]$Run,
    [string]$Branding
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-BrandValue([object]$Document, [string]$Name) {
    if (-not $Document) { return $null }
    $property = $Document.PSObject.Properties[$Name]
    if (-not $property) { return $null }
    $value = [string]$property.Value
    if ([string]::IsNullOrWhiteSpace($value)) { return $null }
    return $value.Trim()
}

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'scripts/package-windows.ps1 is intended for native Windows packaging.'
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
if ($Branding) {
    $resolvedBranding = (Resolve-Path -LiteralPath $Branding -ErrorAction Stop).Path
    $brandDocument = Get-Content -Raw -LiteralPath $resolvedBranding | ConvertFrom-Json
    $brandFormat = Get-BrandValue $brandDocument 'format'
    if ($brandFormat -and $brandFormat -ne 'application-brand') {
        throw 'Unsupported application branding format.'
    }
    $env:APPLICATION_BRAND = $resolvedBranding
}
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
$rustHost = (& rustc -vV | Select-String '^host:' | ForEach-Object { $_.Line.Substring(5).Trim() })
if (-not $rustHost) { throw 'Could not determine the native Rust host triple.' }
$importLibraryNames = if ($rustHost -like '*-msvc') { @('mpv.lib') } else { @('libmpv.dll.a', 'libmpv.a') }
$libmpvImportLibrary = $importLibraryNames |
    ForEach-Object { Join-Path $libmpvDirectory $_ } |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
    Select-Object -First 1
if (-not $libmpvImportLibrary) {
    throw "Required libmpv import library for $rustHost is missing. Expected one of: $($importLibraryNames -join ', ') in $libmpvDirectory"
}
$libmpvRuntime = @('libmpv-2.dll', 'mpv-2.dll') |
    ForEach-Object { Join-Path $libmpvDirectory $_ } |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
    Select-Object -First 1
if (-not $libmpvRuntime) {
    throw "Required libmpv runtime is missing. Expected libmpv-2.dll or mpv-2.dll in $libmpvDirectory"
}
$env:Path = $libmpvDirectory + ';' + $env:Path
$upxCommand = Get-Command upx.exe -ErrorAction SilentlyContinue
$upxPath = if ($upxCommand) { $upxCommand.Source } else { $null }
if (-not $upxPath) {
    $programFilesUpx = Join-Path $env:ProgramFiles 'UPX\upx.exe'
    if (Test-Path -LiteralPath $programFilesUpx -PathType Leaf) {
        $upxPath = $programFilesUpx
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
$effectiveExecutableName = if ($env:APP_EXECUTABLE_NAME) {
    $env:APP_EXECUTABLE_NAME.Trim()
} elseif ($Branding -and (Get-BrandValue $brandDocument 'executableName')) {
    Get-BrandValue $brandDocument 'executableName'
} else {
    'pealayer'
}
if ($effectiveExecutableName -notmatch '^[A-Za-z0-9_.-]+$' -or $effectiveExecutableName.Contains('..')) {
    throw 'Brand executableName must be a safe extension-free file name.'
}
$effectiveExecutableFile = "$effectiveExecutableName.exe"
$stagedExecutable = Join-Path $stagingDirectory $effectiveExecutableFile
$stagedRuntime = Join-Path $stagingDirectory 'libmpv-2.dll'
Copy-Item -LiteralPath (Join-Path $releaseDirectory 'pealayer.exe') -Destination $stagedExecutable -Force
Copy-Item -LiteralPath $libmpvRuntime -Destination $stagedRuntime -Force

$resource = (Get-Item -LiteralPath $stagedExecutable).VersionInfo
$expectedProductName = if ($env:APP_NAME) {
    $env:APP_NAME.Trim()
} elseif ($Branding -and (Get-BrandValue $brandDocument 'applicationName')) {
    Get-BrandValue $brandDocument 'applicationName'
} else {
    'Pealayer'
}
if ($resource.ProductName -ne $expectedProductName -or $resource.OriginalFilename -ne $effectiveExecutableFile) {
    throw 'Packaged executable is missing the expected Win32 identity resources.'
}

$unpackedBytes = (Get-Item -LiteralPath $stagedExecutable).Length
$upxVersion = $null
if (-not $NoUpx) {
    if (-not $upxPath) { throw 'UPX is required for Windows packaging; install it system-wide or pass -NoUpx explicitly.' }
    $upxVersion = (& $upxPath --version | Select-Object -First 1)
    & $upxPath --best --lzma $stagedExecutable
    if ($LASTEXITCODE -ne 0) { throw "UPX compression failed with exit code $LASTEXITCODE" }
    & $upxPath -t $stagedExecutable
    if ($LASTEXITCODE -ne 0) { throw "UPX validation failed with exit code $LASTEXITCODE" }
}

$env:Path = $stagingDirectory + ';' + $libmpvDirectory + ';' + $env:Path
$smoke = Start-Process -FilePath $stagedExecutable -ArgumentList '--smoke-test' -WorkingDirectory $stagingDirectory -Wait -PassThru
if ($smoke.ExitCode -ne 0) { throw "Packaged Pealayer/libmpv smoke test failed with exit code $($smoke.ExitCode)" }

Copy-Item -LiteralPath $stagedExecutable -Destination $outputDirectory -Force
Copy-Item -LiteralPath $stagedRuntime -Destination $outputDirectory -Force
$fontSource = Join-Path $repositoryRoot 'assets\fonts\Vazirmatn-Regular.ttf'
if (-not (Test-Path -LiteralPath $fontSource -PathType Leaf)) {
    throw "Bundled Persian fallback font is missing: $fontSource"
}
$fontDirectory = Join-Path $outputDirectory 'assets\fonts'
New-Item -ItemType Directory -Force -Path $fontDirectory | Out-Null
$packagedFont = Join-Path $fontDirectory 'Vazirmatn-Regular.ttf'
Copy-Item -LiteralPath $fontSource -Destination $packagedFont -Force
$webDistribution = Join-Path $repositoryRoot 'web_ui\dist'
$webUiPackaged = $false
if (Test-Path -LiteralPath (Join-Path $webDistribution 'index.html')) {
    $packagedWebDistribution = Join-Path $outputDirectory 'web_ui\dist'
    New-Item -ItemType Directory -Force -Path $packagedWebDistribution | Out-Null
    Copy-Item -Path (Join-Path $webDistribution '*') -Destination $packagedWebDistribution -Recurse -Force
    $webUiPackaged = $true
}

$artifacts = @($effectiveExecutableFile,'libmpv-2.dll','assets/fonts/Vazirmatn-Regular.ttf') | ForEach-Object {
    $path = Join-Path $outputDirectory $_
    [ordered]@{
        path = $_
        bytes = (Get-Item -LiteralPath $path).Length
        sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
    }
}
$manifest = [ordered]@{
    format = 'pealayer-windows-package'
    version = $resource.ProductVersion
    git_commit = (& git -C $repositoryRoot rev-parse HEAD).Trim()
    git_dirty = [bool](& git -C $repositoryRoot status --porcelain)
    built_at_utc = [DateTime]::UtcNow.ToString('o')
    target = (& rustc -vV | Select-String '^host:' | ForEach-Object { $_.Line.Substring(5).Trim() })
    identity = [ordered]@{
        format = 'application-brand'
        application_name = $resource.ProductName
        company_name = $resource.CompanyName
        file_description = $resource.FileDescription
        legal_copyright = $resource.LegalCopyright
        executable_name = $effectiveExecutableName
    }
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
    Start-Process -FilePath (Join-Path $outputDirectory $effectiveExecutableFile) -WorkingDirectory $outputDirectory
}
