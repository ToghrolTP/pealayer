[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$SourcePackage,
    [Parameter(Mandatory = $true)][ValidatePattern('^[a-z0-9][a-z0-9-]{0,63}$')][string]$CandidateName,
    [Parameter(Mandatory = $true)][string]$RetainedRuntime,
    [Parameter(Mandatory = $true)][ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedRuntimeSha256
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'libmpv-windows.ps1')
. (Join-Path $PSScriptRoot 'package-probe-windows.ps1')

$sourceRoot = (Resolve-Path -LiteralPath $SourcePackage).Path.TrimEnd('\', '/')
$stageRoot = Split-Path -Parent $sourceRoot
$candidateRoot = [IO.Path]::GetFullPath((Join-Path $stageRoot $CandidateName))
if ($candidateRoot -ieq $sourceRoot -or
    -not $candidateRoot.StartsWith($stageRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Host candidate must be a separate named sibling of the source package.'
}
$manifest = Get-Content -Raw -LiteralPath (Join-Path $sourceRoot 'host-manifest.json') | ConvertFrom-Json
if ($manifest.format -ne 'pealayer-windows-package' -or $manifest.git_dirty) {
    throw 'A verified clean-source Windows package is required.'
}
foreach ($artifact in $manifest.artifacts) {
    $path = [IO.Path]::GetFullPath((Join-Path $sourceRoot $artifact.path))
    if (-not $path.StartsWith($sourceRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $artifact.sha256) {
        throw "Source artifact failed integrity: $($artifact.path)"
    }
}
$runtimePath = (Resolve-Path -LiteralPath $RetainedRuntime).Path
$runtimeHash = (Get-FileHash -LiteralPath $runtimePath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($runtimeHash -ne $ExpectedRuntimeSha256.ToLowerInvariant()) {
    throw 'Retained destination runtime identity mismatch; refusing host candidate.'
}
if (Test-Path -LiteralPath $candidateRoot) {
    throw 'Candidate already exists; preserve/review it before preparing a replacement.'
}
New-Item -ItemType Directory -Path $candidateRoot | Out-Null
foreach ($relative in @('pealayer.exe', 'pealayer-downloader.exe', 'assets/fonts/Vazirmatn-Regular.ttf')) {
    [void](Copy-PealayerFileLowWrite -Source (Join-Path $sourceRoot $relative) -Destination (Join-Path $candidateRoot $relative) -PreferHardLink)
}
foreach ($alias in @('libmpv-2.dll', 'mpv-2.dll')) {
    [void](Copy-PealayerFileLowWrite -Source $runtimePath -Destination (Join-Path $candidateRoot $alias) -PreferHardLink)
}
Copy-Item -LiteralPath (Join-Path $sourceRoot 'web_ui') -Destination $candidateRoot -Recurse

[void](Invoke-PackageProbe (Join-Path $candidateRoot 'pealayer.exe') '--smoke-test' $candidateRoot)
$identity = Invoke-PackageProbe (Join-Path $candidateRoot 'pealayer.exe') '--build-info' $candidateRoot | ConvertFrom-Json
if ($identity.commit -ne $manifest.git_commit -or $identity.dirty) {
    throw 'Host candidate executable does not match the clean source package.'
}
[void](Invoke-PackageProbe (Join-Path $candidateRoot 'pealayer-downloader.exe') '--smoke-test' $candidateRoot)
# Build provenance stays factual: the compiler's runtime profile is not replaced
# by the host pairing. Record the separately verified deployment runtime.
$manifest | Add-Member -NotePropertyName deployment_runtime -NotePropertyValue (Get-PealayerFileIdentity -Path $runtimePath)
$manifest.validation | Add-Member -NotePropertyName retained_runtime_smoke -NotePropertyValue 'passed'
$manifest.artifacts = @($manifest.artifacts | ForEach-Object {
    $path = Join-Path $candidateRoot $_.path
    [ordered]@{ path = $_.path; bytes = (Get-Item -LiteralPath $path).Length;
        sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() }
})
$manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $candidateRoot 'host-manifest.json') -Encoding utf8
[pscustomobject]@{ candidate = $candidateRoot; commit = $identity.commit; runtime_sha256 = $runtimeHash; smoke = 'passed' }
