[CmdletBinding()]
param(
    [switch]$NoUpx,
    [switch]$SkipTests,
    [switch]$Run,
    [string]$Branding,
    [string]$LibmpvDirectory,
    [string]$OutputDirectory
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
. (Join-Path $PSScriptRoot 'libmpv-windows.ps1')
$packageCommit = (& git -C $repositoryRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or !$packageCommit) { throw 'Cannot identify package source.' }
if (& git -C $repositoryRoot status --porcelain) { throw 'Commit or preserve local changes before packaging a production build.' }
# Pin the existing build-metadata contract, invalidating stale shared-cache
# metadata even when the worktree commit changed without a source-file change.
$env:GITHUB_SHA = $packageCommit
$env:GITHUB_REF_NAME = (& git -C $repositoryRoot rev-parse --abbrev-ref HEAD).Trim()
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
$cargoTargetDirectory = if ($env:CARGO_TARGET_DIR) {
    if ([System.IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
        [System.IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
    } else {
        [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot $env:CARGO_TARGET_DIR))
    }
} else {
    Join-Path $repositoryRoot 'target'
}
$releaseDirectory = Join-Path $cargoTargetDirectory 'release'
$stagingDirectory = Join-Path $cargoTargetDirectory 'package-windows'
$sourceDirectory = Split-Path -Parent $repositoryRoot
$outputDirectory = if ($OutputDirectory) {
    [System.IO.Path]::GetFullPath($OutputDirectory)
} elseif ((Split-Path -Leaf $sourceDirectory) -ieq 'source') {
    Join-Path (Split-Path -Parent $sourceDirectory) 'bin'
} else {
    Join-Path $repositoryRoot 'bin'
}
$rustHost = (& rustc -vV | Select-String '^host:' | ForEach-Object { $_.Line.Substring(5).Trim() })
if (-not $rustHost) { throw 'Could not determine the native Rust host triple.' }
$libmpv = Resolve-PealayerLibmpv -RepositoryRoot $repositoryRoot -RustHost $rustHost -ExplicitDirectory $LibmpvDirectory
$libmpvSourceDirectory = $libmpv.SourceDirectory
$libmpvDirectory = $libmpv.LinkDirectory
$libmpvImportLibrary = $libmpv.ImportLibrary
$libmpvRuntime = $libmpv.RuntimeLibrary
$hostProfile = Save-PealayerWindowsHostProfile -RepositoryRoot $repositoryRoot -Resolution $libmpv -PersistUserEnvironment
Set-PealayerLibmpvBuildEnvironment -Resolution $libmpv
$upxCommand = Get-Command upx.exe -ErrorAction SilentlyContinue
$upxPath = if ($upxCommand) { $upxCommand.Source } else { $null }
if (-not $upxPath) {
    $programFilesUpx = Join-Path $env:ProgramFiles 'UPX\upx.exe'
    if (Test-Path -LiteralPath $programFilesUpx -PathType Leaf) {
        $upxPath = $programFilesUpx
    }
}

if (-not $SkipTests) {
    & cargo test --locked --jobs 1 -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed with exit code $LASTEXITCODE" }
}

$webUiDirectory = Join-Path $repositoryRoot 'web_ui'
$webUiPackage = Join-Path $webUiDirectory 'package.json'
if (Test-Path -LiteralPath $webUiPackage -PathType Leaf) {
    $npm = Get-Command npm.cmd -ErrorAction SilentlyContinue
    if (-not $npm) { $npm = Get-Command npm -ErrorAction SilentlyContinue }
    if (-not $npm) { throw 'npm is required to build the Pealayer Web UI.' }
    Push-Location -LiteralPath $webUiDirectory
    try {
        & $npm.Source run build
        if ($LASTEXITCODE -ne 0) { throw "Web UI build failed with exit code $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
}

& (Join-Path $PSScriptRoot 'run-windows.ps1') -BuildOnly -LibmpvDirectory $libmpvSourceDirectory

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
Copy-PealayerLibmpvRuntime -RuntimeLibrary $libmpvRuntime -DestinationDirectory $stagingDirectory

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
& (Join-Path $PSScriptRoot 'verify-windows-quick-action-icons.ps1') -Executable $stagedExecutable

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

$identityOutput = Join-Path $stagingDirectory 'build-identity.json'
$identityProcess = Start-Process -FilePath $stagedExecutable -ArgumentList '--build-info' -WorkingDirectory $stagingDirectory -WindowStyle Hidden -RedirectStandardOutput $identityOutput -Wait -PassThru
if ($identityProcess.ExitCode -ne 0) { throw 'Cannot inspect the packaged executable build identity.' }
$embeddedIdentity = Get-Content -Raw -LiteralPath $identityOutput | ConvertFrom-Json
if ($embeddedIdentity.commit -ne $packageCommit -or $embeddedIdentity.dirty) {
    throw 'Embedded executable identity does not match clean package source; refusing publication.'
}
if ((& git -C $repositoryRoot rev-parse HEAD).Trim() -ne $packageCommit -or (& git -C $repositoryRoot status --porcelain)) {
    throw 'Package source changed during the build; refusing publication.'
}

Copy-Item -LiteralPath $stagedExecutable -Destination $outputDirectory -Force
Copy-PealayerLibmpvRuntime -RuntimeLibrary $stagedRuntime -DestinationDirectory $outputDirectory
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
    $resolvedOutputDirectory = [System.IO.Path]::GetFullPath($outputDirectory).TrimEnd('\', '/')
    $resolvedPackagedWebDistribution = [System.IO.Path]::GetFullPath($packagedWebDistribution)
    if (-not $resolvedPackagedWebDistribution.StartsWith(
        $resolvedOutputDirectory + [System.IO.Path]::DirectorySeparatorChar,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw 'Refusing to replace a Web UI package outside the canonical output directory.'
    }
    if (Test-Path -LiteralPath $resolvedPackagedWebDistribution) {
        Remove-Item -LiteralPath $resolvedPackagedWebDistribution -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $packagedWebDistribution | Out-Null
    Copy-Item -Path (Join-Path $webDistribution '*') -Destination $packagedWebDistribution -Recurse -Force
    $webUiPackaged = $true
}

$artifacts = @($effectiveExecutableFile,'libmpv-2.dll','mpv-2.dll','assets/fonts/Vazirmatn-Regular.ttf') | ForEach-Object {
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
    git_commit = $embeddedIdentity.commit
    git_dirty = $embeddedIdentity.dirty
    built_at_utc = [DateTime]::UtcNow.ToString('o')
    target = (& rustc -vV | Select-String '^host:' | ForEach-Object { $_.Line.Substring(5).Trim() })
    build_host = [ordered]@{
        computer_name = $env:COMPUTERNAME
        profile = $hostProfile.ProfilePath
        libmpv_source_directory = $libmpvSourceDirectory
        libmpv_link_directory = $libmpvDirectory
        resolution_source = $libmpv.ResolutionSource
        import_library = Get-PealayerFileIdentity -Path $libmpvImportLibrary
        import_source = Get-PealayerFileIdentity -Path $libmpv.ImportSource
        runtime = Get-PealayerFileIdentity -Path $libmpvRuntime
    }
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
        quick_action_icons = 'verified'
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
