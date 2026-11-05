# Start a real compiled revision with an isolated, disconnected comparison profile.
param(
    [string]$Executable = "$env:LOCALAPPDATA\Programs\Pealayer\build-cache\cargo-target\release\pealayer.exe",
    [string]$Profile = "$env:LOCALAPPDATA\Programs\Pealayer\verification\status-history.json",
    [int]$Port = 18100,
    # Historical builds before explicit config paths use their original portable mode.
    # This is a review harness, not an application compatibility layer.
    [string]$HistoricalPortableDirectory,
    [string]$LaunchArguments = '--workspace nle --pause'
)
$ErrorActionPreference = 'Stop'
$settings = Get-Content -LiteralPath $Profile -Raw | ConvertFrom-Json
if ($settings.auto_connect_hardware -ne $false -or $settings.restore_last_media_on_startup -ne $false) {
    throw 'Comparison profile must disable hardware auto-connect and media restoration.'
}
$start = [Diagnostics.ProcessStartInfo]::new()
$start.FileName = (Resolve-Path -LiteralPath $Executable).Path
$start.WorkingDirectory = Join-Path $env:LOCALAPPDATA 'Programs\Pealayer\bin'
if ($HistoricalPortableDirectory) {
    $portable = (Resolve-Path -LiteralPath $HistoricalPortableDirectory).Path
    if (-not (Test-Path -LiteralPath (Join-Path $portable 'portable.flag'))) {
        throw 'Historical comparison requires an explicit isolated portable.flag.'
    }
    $portableSettings = Get-Content -LiteralPath (Join-Path $portable 'config\settings.json') -Raw | ConvertFrom-Json
    if ($portableSettings.recent_media.Count -ne 0) { throw 'Historical profile must not contain private media.' }
    $start.WorkingDirectory = $portable
    $start.EnvironmentVariables['PATH'] = "$(Join-Path $env:LOCALAPPDATA 'Programs\Pealayer\bin');$env:PATH"
}
$start.UseShellExecute = $false
$start.WindowStyle = [Diagnostics.ProcessWindowStyle]::Hidden
$start.Arguments = $LaunchArguments
$start.EnvironmentVariables['PEALAYER_CONFIG_FILE'] = $Profile
$start.EnvironmentVariables['PEALAYER_INSTANCE_ID'] = 'status-history-review'
$start.EnvironmentVariables['PEALAYER_PORT'] = "$Port"
$start.EnvironmentVariables['PEALAYER_WEB_BIND'] = '127.0.0.1'
$start.EnvironmentVariables['PEALAYER_HTTP_PORT'] = "$Port"
$start.EnvironmentVariables['PEALAYER_WS_PORT'] = "$($Port + 1)"
$start.EnvironmentVariables['APP_THEME'] = 'dark'
$start.EnvironmentVariables['APP_LANGUAGE'] = 'en'
$process = [Diagnostics.Process]::Start($start)
Write-Host "Status review launched as PID $($process.Id). Requested isolated profile; historical revisions may predate the process API/port overrides. Verify that revision's startup contract before use."
