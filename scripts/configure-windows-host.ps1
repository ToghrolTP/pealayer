[CmdletBinding()]
param(
    [string]$LibmpvDirectory,
    [switch]$NoPersistUserEnvironment,
    [switch]$Json
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'scripts/configure-windows-host.ps1 is intended for native Windows hosts.'
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'libmpv-windows.ps1')

$rustHost = (& rustc -vV | Select-String '^host:' | ForEach-Object { $_.Line.Substring(5).Trim() })
if (-not $rustHost) { throw 'Could not determine the native Rust host triple.' }

$resolution = Resolve-PealayerLibmpv `
    -RepositoryRoot $repositoryRoot `
    -RustHost $rustHost `
    -ExplicitDirectory $LibmpvDirectory
$saved = Save-PealayerWindowsHostProfile `
    -RepositoryRoot $repositoryRoot `
    -Resolution $resolution `
    -PersistUserEnvironment:(-not $NoPersistUserEnvironment)
Set-PealayerLibmpvBuildEnvironment -Resolution $resolution

$result = [ordered]@{
    host = $resolution.Host
    rust_host = $resolution.RustHost
    source_directory = $resolution.SourceDirectory
    link_directory = $resolution.LinkDirectory
    resolution_source = $resolution.ResolutionSource
    import_library = Get-PealayerFileIdentity -Path $resolution.ImportLibrary
    runtime = Get-PealayerFileIdentity -Path $resolution.RuntimeLibrary
    host_profile = $saved.ProfilePath
    cargo_config = $saved.CargoConfigPath
    user_environment_persisted = $saved.UserEnvironmentPersisted
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
} else {
    Write-Host "Pealayer Windows host configured: $($resolution.Host)"
    Write-Host "  libmpv source : $($resolution.SourceDirectory)"
    Write-Host "  linker path   : $($resolution.LinkDirectory)"
    Write-Host "  runtime       : $($result.runtime.file_name) $($result.runtime.file_version)"
    Write-Host "  runtime SHA256: $($result.runtime.sha256)"
    Write-Host "  host profile  : $($saved.ProfilePath)"
    Write-Host "  Cargo config  : $($saved.CargoConfigPath)"
}
