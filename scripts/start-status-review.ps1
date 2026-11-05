# Start a real compiled revision with an isolated, disconnected comparison profile.
param(
    [string]$Executable = "$env:LOCALAPPDATA\Programs\Pealayer\build-cache\cargo-target\release\pealayer.exe",
    [string]$Profile = "$env:LOCALAPPDATA\Programs\Pealayer\verification\status-history.json",
    [int]$Port = 18100
)
$ErrorActionPreference = 'Stop'
$settings = Get-Content -LiteralPath $Profile -Raw | ConvertFrom-Json
if ($settings.auto_connect_hardware -ne $false -or $settings.restore_last_media_on_startup -ne $false) {
    throw 'Comparison profile must disable hardware auto-connect and media restoration.'
}
$start = [Diagnostics.ProcessStartInfo]::new()
$start.FileName = (Resolve-Path -LiteralPath $Executable).Path
$start.WorkingDirectory = Join-Path $env:LOCALAPPDATA 'Programs\Pealayer\bin'
$start.UseShellExecute = $false
$start.WindowStyle = [Diagnostics.ProcessWindowStyle]::Hidden
$start.Arguments = '--workspace nle --pause'
$start.EnvironmentVariables['PEALAYER_CONFIG_FILE'] = $Profile
$start.EnvironmentVariables['PEALAYER_INSTANCE_ID'] = 'status-history-review'
$start.EnvironmentVariables['PEALAYER_PORT'] = "$Port"
$start.EnvironmentVariables['PEALAYER_WEB_BIND'] = '127.0.0.1'
$process = [Diagnostics.Process]::Start($start)
Write-Host "Status review launched as PID $($process.Id), loopback port $Port. No production settings or hardware connection."
