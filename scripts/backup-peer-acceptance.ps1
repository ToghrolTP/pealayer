[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][uri]$Endpoint,
    [Parameter(Mandatory = $true)][ValidatePattern('^[a-z0-9][a-z0-9-]{0,63}$')][string]$BackupName
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($Endpoint.Scheme -ne 'http' -or -not $Endpoint.IsLoopback) {
    throw 'Run this backup on the destination against its own loopback control endpoint.'
}
$base = $Endpoint.AbsoluteUri.TrimEnd('/')
$process = Invoke-RestMethod -Uri "$base/api/process/status" -TimeoutSec 8
$player = Invoke-RestMethod -Uri "$base/api/player/status" -TimeoutSec 8
if ($player.playing -or ($null -ne $player.hardware_sync -and $player.hardware_sync.play_requested)) {
    throw 'Acceptance updates require paused media and no pending Play intent.'
}
$manifest = Invoke-RestMethod -Uri "$base/api/update/manifest" -TimeoutSec 8
$root = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Programs\Pealayer\verification'
$backup = [IO.Path]::GetFullPath((Join-Path $root $BackupName))
if (-not $backup.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Backup escaped the private program verification directory.'
}
if (Test-Path -LiteralPath $backup) { throw 'Named backup already exists; do not overwrite it.' }
New-Item -ItemType Directory -Path $backup -Force | Out-Null
foreach ($entry in @(
    @{Name='process-status.json'; Value=$process},
    @{Name='player-status.json'; Value=$player},
    @{Name='update-manifest.json'; Value=$manifest},
    @{Name='config.json'; Value=(Invoke-RestMethod -Uri "$base/api/config" -TimeoutSec 8)}
)) {
    $path = Join-Path $backup $entry.Name
    $entry.Value | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $path -Encoding utf8
    # Parse the saved data before reporting backup success, without displaying
    # private media/configuration or sending it to GitHub.
    $null = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
    [pscustomobject]@{file=$entry.Name; sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
}
[pscustomobject]@{backup=$backup; process_id=$process.process_id; commit=$process.git_commit; executable_sha256=$manifest.sha256}
