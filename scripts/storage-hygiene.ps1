[CmdletBinding()]
param(
    [switch]$PruneRepositoryTarget,
    [double]$SharedCacheWarningGiB = 48
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repositoryRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'libmpv-windows.ps1')
$paths = Get-PealayerWindowsHostPaths -RepositoryRoot $repositoryRoot
$repositoryTarget = Join-Path $repositoryRoot 'target'

function Get-DirectorySummary([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        return [pscustomobject]@{ Path = $Path; Files = 0; GiB = 0; Exists = $false }
    }
    $measurement = Get-ChildItem -LiteralPath $Path -File -Recurse -Force -ErrorAction SilentlyContinue |
        Measure-Object -Property Length -Sum
    [pscustomobject]@{
        Path = $Path
        Files = $measurement.Count
        GiB = [math]::Round($measurement.Sum / 1GB, 3)
        Exists = $true
    }
}

$repositorySummary = Get-DirectorySummary -Path $repositoryTarget
$sharedSummary = Get-DirectorySummary -Path $paths.CargoTargetDirectory
$stagingSummary = Get-DirectorySummary -Path (Join-Path $paths.ProgramRoot 'staging')
@($repositorySummary, $sharedSummary, $stagingSummary) | Format-Table -AutoSize

if ($sharedSummary.GiB -gt $SharedCacheWarningGiB) {
    Write-Warning "Shared Cargo output is $($sharedSummary.GiB) GiB; inspect profiles before an explicit Cargo clean."
}
if (-not $PruneRepositoryTarget -or -not $repositorySummary.Exists) { return }

$resolvedRepository = (Resolve-Path -LiteralPath $repositoryRoot).Path.TrimEnd('\', '/')
$resolvedTarget = (Resolve-Path -LiteralPath $repositoryTarget).Path
if ($resolvedTarget -ne ($resolvedRepository + [System.IO.Path]::DirectorySeparatorChar + 'target')) {
    throw "Refusing unexpected repository target path: $resolvedTarget"
}
& git -C $repositoryRoot check-ignore --quiet -- target
if ($LASTEXITCODE -ne 0) { throw 'Refusing to prune a repository target that Git does not ignore.' }

$allowedChildren = @(
    'debug', 'release', 'x86_64-pc-windows-msvc', 'x86_64-pc-windows-gnu',
    'package-windows', 'mpv-msvc-import', 'mpv-import', 'tmp',
    '.rustc_info.json', 'CACHEDIR.TAG'
)
$unexpected = @(Get-ChildItem -LiteralPath $resolvedTarget -Force | Where-Object { $_.Name -notin $allowedChildren })
if ($unexpected.Count -ne 0) {
    throw "Move non-Cargo evidence out of target before pruning: $($unexpected.Name -join ', ')"
}
$active = @(Get-CimInstance Win32_Process | Where-Object {
    $_.ExecutablePath -and $_.ExecutablePath.StartsWith(
        $resolvedTarget + [System.IO.Path]::DirectorySeparatorChar,
        [System.StringComparison]::OrdinalIgnoreCase
    )
})
if ($active.Count -ne 0) {
    throw "Repository target is in use by process IDs: $($active.ProcessId -join ', ')"
}
Remove-Item -LiteralPath $resolvedTarget -Recurse -Force
Write-Host "Pruned obsolete repository-local Cargo output: $resolvedTarget"
