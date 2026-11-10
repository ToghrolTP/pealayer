# Launch only the explicitly isolated, hardware-disconnected review profile.
$ErrorActionPreference = 'Stop'
$programRoot = Join-Path $env:LOCALAPPDATA 'Programs\Pealayer'
$reviewConfig = Join-Path $programRoot 'verification\ui-modes-review.json'
$reviewExecutable = Join-Path $programRoot 'build-cache\cargo-target\release\pealayer.exe'
if (-not (Test-Path -LiteralPath $reviewExecutable)) { throw 'Build the review executable first.' }
$profile = Get-Content -LiteralPath $reviewConfig -Raw | ConvertFrom-Json
if ($profile.auto_connect_hardware -ne $false) { throw 'Review profile must disable automatic hardware connection.' }
$names = @('PEALAYER_CONFIG_FILE', 'PEALAYER_INSTANCE_ID', 'PEALAYER_PORT', 'PEALAYER_WEB_BIND')
$previous = @{}
foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
try {
    $env:PEALAYER_CONFIG_FILE = $reviewConfig
    $env:PEALAYER_INSTANCE_ID = 'ui-modes-review'
    $env:PEALAYER_PORT = '18090'
    $env:PEALAYER_WEB_BIND = '127.0.0.1'
    $process = Start-Process -FilePath $reviewExecutable -WorkingDirectory (Join-Path $programRoot 'bin') -ArgumentList '--workspace nle --pause --activate' -WindowStyle Hidden -PassThru
    Write-Host "Review launched as process $($process.Id). Preferences will be navigated through its API."
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
}
