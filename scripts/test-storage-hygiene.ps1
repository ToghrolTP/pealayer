[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Small generated fixtures only: no real repository, cache, or compiler is touched.
$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('pealayer-hygiene-test-' + [guid]::NewGuid())
$originalLocalAppData = $env:LOCALAPPDATA
$fixtureCache = Join-Path $fixtureRoot 'Programs\Pealayer\build-cache\cargo-target'
$effectiveFixtureCache = $fixtureCache
$fixtureActive = @()
$fixtureIgnored = $true
function cargo { $global:LASTEXITCODE = 0; @{ target_directory = $effectiveFixtureCache } | ConvertTo-Json }
function git { $global:LASTEXITCODE = if ($fixtureIgnored) { 0 } else { 1 } }
function Get-CimInstance { param($ClassName); $fixtureActive }
function Assert-Refusal([scriptblock]$Action, [string]$Expected) {
    $refused = $false
    try { & $Action | Out-Null } catch {
        if ($_.Exception.Message -notmatch $Expected) { throw }
        $refused = $true
    }
    if (-not $refused) { throw "Expected safety refusal: $Expected" }
}
function New-FixtureFile([string]$Path, [string]$Value = 'generated test fixture') {
    New-Item -ItemType Directory -Path (Split-Path -Parent $Path) -Force | Out-Null
    [System.IO.File]::WriteAllText($Path, $Value)
}

try {
    $env:LOCALAPPDATA = $fixtureRoot
    $fixtureScripts = Join-Path $fixtureRoot 'repo\scripts'
    New-Item -ItemType Directory -Path $fixtureScripts -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'storage-hygiene.ps1') -Destination $fixtureScripts
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'libmpv-windows.ps1') -Destination $fixtureScripts
    $hygiene = Join-Path $fixtureScripts 'storage-hygiene.ps1'
    $target = Join-Path $fixtureRoot 'repo\target'
    $cacheTag = Join-Path $fixtureCache 'CACHEDIR.TAG'
    New-FixtureFile $cacheTag 'Signature: 8a477f597d28d172789f06886806bc55'
    New-FixtureFile (Join-Path $fixtureCache 'release\preserve.rlib')
    New-FixtureFile (Join-Path $target 'debug\deps\obsolete.rlib')
    New-FixtureFile (Join-Path $target 'x86_64-pc-windows-msvc\debug\obsolete.pdb')
    New-FixtureFile (Join-Path $target 'release\preserve.exe')
    New-FixtureFile (Join-Path $target 'x86_64-pc-windows-msvc\release\preserve.exe')
    New-FixtureFile (Join-Path $target 'package-windows\preserve.exe')

    & $hygiene -PruneRepositoryDebug -WhatIf | Out-Null
    if (-not (Test-Path (Join-Path $target 'debug\deps\obsolete.rlib'))) { throw 'WhatIf deleted data.' }
    Assert-Refusal { & $hygiene -PruneRepositoryDebug -PruneRepositoryTarget } 'not both'
    $fixtureIgnored = $false
    Assert-Refusal { & $hygiene -PruneRepositoryDebug } 'Git does not ignore'
    $fixtureIgnored = $true
    $effectiveFixtureCache = $target
    Assert-Refusal { & $hygiene -PruneRepositoryDebug } 'not using the canonical shared cache'
    $effectiveFixtureCache = $fixtureCache
    New-FixtureFile $cacheTag 'invalid cache tag'
    Assert-Refusal { & $hygiene -PruneRepositoryDebug } 'established shared Cargo cache'
    New-FixtureFile $cacheTag 'Signature: 8a477f597d28d172789f06886806bc55'
    $fixtureActive = @([pscustomobject]@{ Name = 'rustc.exe'; ProcessId = 123; ExecutablePath = 'C:\toolchain\rustc.exe' })
    Assert-Refusal { & $hygiene -PruneRepositoryDebug } 'compiler or target executable is active'
    $fixtureActive = @([pscustomobject]@{ Name = 'pealayer.exe'; ProcessId = 124; ExecutablePath = (Join-Path $target 'debug\pealayer.exe') })
    Assert-Refusal { & $hygiene -PruneRepositoryDebug } 'compiler or target executable is active'
    $fixtureActive = @()
    $evidence = Join-Path $target 'owner-notes.txt'
    New-FixtureFile $evidence
    Assert-Refusal { & $hygiene -PruneRepositoryDebug } 'non-Cargo evidence'
    Remove-Item -LiteralPath $evidence

    $link = Join-Path $target 'debug\other-owner'
    New-Item -ItemType Junction -Path $link -Target $fixtureCache | Out-Null
    try { Assert-Refusal { & $hygiene -PruneRepositoryDebug } 'linked content' }
    finally { Remove-Item -LiteralPath $link -Force }
    $architectureDebug = Join-Path $target 'x86_64-pc-windows-msvc\debug'
    Remove-Item -LiteralPath $architectureDebug -Recurse -Force
    New-Item -ItemType Junction -Path $architectureDebug -Target $fixtureCache | Out-Null
    try { Assert-Refusal { & $hygiene -PruneRepositoryDebug } 'linked prune path' }
    finally { Remove-Item -LiteralPath $architectureDebug -Force }
    New-FixtureFile (Join-Path $architectureDebug 'obsolete.pdb')

    & $hygiene -PruneRepositoryDebug -Confirm:$false | Out-Null
    foreach ($path in @('debug', 'x86_64-pc-windows-msvc\debug')) {
        if (Test-Path (Join-Path $target $path)) { throw "Debug fixture not pruned: $path" }
    }
    foreach ($path in @('release\preserve.exe', 'x86_64-pc-windows-msvc\release\preserve.exe', 'package-windows\preserve.exe')) {
        if (-not (Test-Path (Join-Path $target $path))) { throw "Protected fixture deleted: $path" }
    }
    if (-not (Test-Path (Join-Path $fixtureCache 'release\preserve.rlib'))) { throw 'Shared cache modified.' }
    Write-Host 'Storage hygiene safety tests passed (dry run, cache, ignore, writer, evidence, links, debug-only preservation).'
} finally {
    $env:LOCALAPPDATA = $originalLocalAppData
    $resolvedFixture = [System.IO.Path]::GetFullPath($fixtureRoot)
    $tempPrefix = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\') + '\pealayer-hygiene-test-'
    if (-not $resolvedFixture.StartsWith($tempPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing unexpected fixture cleanup path.'
    }
    if (Test-Path -LiteralPath $resolvedFixture) { Remove-Item -LiteralPath $resolvedFixture -Recurse -Force }
}
