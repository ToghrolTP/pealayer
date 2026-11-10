[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [switch]$PruneRepositoryTarget,
    [switch]$PruneRepositoryDebug,
    [double]$SharedCacheWarningGiB = 48
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repositoryRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'libmpv-windows.ps1')
$paths = Get-PealayerWindowsHostPaths -RepositoryRoot $repositoryRoot
$repositoryTarget = Join-Path $repositoryRoot 'target'
if ($PruneRepositoryTarget -and $PruneRepositoryDebug) {
    throw 'Select debug-only pruning or whole-target pruning, not both.'
}

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
if ((-not $PruneRepositoryTarget -and -not $PruneRepositoryDebug) -or -not $repositorySummary.Exists) { return }

$resolvedRepository = (Resolve-Path -LiteralPath $repositoryRoot).Path.TrimEnd('\', '/')
$resolvedTarget = (Resolve-Path -LiteralPath $repositoryTarget).Path
if ($resolvedTarget -ne ($resolvedRepository + [System.IO.Path]::DirectorySeparatorChar + 'target')) {
    throw "Refusing unexpected repository target path: $resolvedTarget"
}
if ((Get-Item -LiteralPath $resolvedTarget -Force).Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
    throw 'Refusing a repository target that is a junction or symbolic link.'
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
$cacheTag = Join-Path $paths.CargoTargetDirectory 'CACHEDIR.TAG'
if (-not (Test-Path -LiteralPath $cacheTag -PathType Leaf) -or
    (Get-Content -LiteralPath $cacheTag -TotalCount 1) -ne 'Signature: 8a477f597d28d172789f06886806bc55') {
    throw 'Refusing to prune without an established shared Cargo cache.'
}
# Ask Cargo itself: a resolver default alone does not prove effective config/env.
$metadataJson = & cargo metadata --offline --locked --no-deps --format-version 1 --manifest-path (Join-Path $repositoryRoot 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'Cannot verify the active Cargo target directory.' }
$effectiveTarget = [System.IO.Path]::GetFullPath(($metadataJson | ConvertFrom-Json).target_directory).TrimEnd('\', '/')
$expectedSharedTarget = [System.IO.Path]::GetFullPath($paths.CargoTargetDirectory).TrimEnd('\', '/')
if ($effectiveTarget -ine $expectedSharedTarget) {
    throw "Refusing to prune: Cargo is not using the canonical shared cache: $effectiveTarget"
}
$prunePaths = if ($PruneRepositoryDebug) {
    @('debug', 'x86_64-pc-windows-msvc\debug', 'x86_64-pc-windows-gnu\debug') |
        ForEach-Object { Join-Path $resolvedTarget $_ } |
        Where-Object { Test-Path -LiteralPath $_ -PathType Container }
} else { @($resolvedTarget) }
foreach ($prunePath in $prunePaths) {
    # Validate every ancestor and descendant; never cross a junction into other work.
    $ancestor = Get-Item -LiteralPath $prunePath -Force
    while ($ancestor.FullName -ine $resolvedTarget) {
        if ($ancestor.Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
            throw "Refusing a linked prune path: $($ancestor.FullName)"
        }
        $ancestor = $ancestor.Parent
    }
    $linked = @(Get-ChildItem -LiteralPath $prunePath -Recurse -Force -ErrorAction Stop |
        Where-Object { $_.Attributes -band [System.IO.FileAttributes]::ReparsePoint })
    if ($linked.Count -ne 0) { throw "Refusing linked content inside: $prunePath" }
}
$active = @(Get-CimInstance Win32_Process | Where-Object {
    # A Cargo/rustc executable lives outside target; path-only checks miss writers.
    $_.Name -in @('cargo.exe', 'rustc.exe', 'rustdoc.exe', 'link.exe') -or
    ($_.ExecutablePath -and $_.ExecutablePath.StartsWith(
        $resolvedTarget + [System.IO.Path]::DirectorySeparatorChar,
        [System.StringComparison]::OrdinalIgnoreCase
    ))
})
if ($active.Count -ne 0) {
    throw "Refusing pruning while a compiler or target executable is active: $($active.ProcessId -join ', ')"
}
foreach ($prunePath in $prunePaths) {
    if ($PSCmdlet.ShouldProcess($prunePath, 'Permanently remove obsolete repository-local Cargo output')) {
        Remove-Item -LiteralPath $prunePath -Recurse -Force
        Write-Host "Pruned obsolete repository-local Cargo output: $prunePath"
    }
}
