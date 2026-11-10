# Read the same global Windows session surface KDE Connect consumes.
# Run with Windows PowerShell 5.1 (WinRT projection), not PowerShell 7.
# This never sends media commands or exposes titles/paths/artwork bytes.
[CmdletBinding()]
param(
    [string]$ApplicationId = 'Pealayer.Desktop',
    [string]$ApiBase = 'http://127.0.0.1:8080'
)
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSEdition -eq 'Core') { throw 'Run this read-only probe with powershell.exe -NoProfile -File.' }
$uri = [Uri]$ApiBase
if (-not $uri.IsLoopback -or $uri.Scheme -notin @('http', 'https')) { throw 'Compare only a local Pealayer API with local OS sessions.' }
$process = Invoke-RestMethod -Uri ($ApiBase.TrimEnd('/') + '/api/process/status') -TimeoutSec 5
$state = Invoke-RestMethod -Uri ($ApiBase.TrimEnd('/') + '/api/player/status') -TimeoutSec 5
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager, Windows.Media.Control, ContentType = WindowsRuntime]
$asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
} | Select-Object -First 1
function Wait-OsRead($Operation, $ResultType) {
    $task = $asTask.MakeGenericMethod($ResultType).Invoke($null, @($Operation))
    if (-not $task.Wait(5000)) { throw 'OS media-session read timed out.' }
    return $task.Result
}
$manager = Wait-OsRead ([Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager]::RequestAsync()) ([Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager])
$sessions = @(foreach ($session in $manager.GetSessions()) {
    if ($session.SourceAppUserModelId -ne $ApplicationId) { continue }
    $info = $session.GetPlaybackInfo()
    $timeline = $session.GetTimelineProperties()
    $metadata = Wait-OsRead ($session.TryGetMediaPropertiesAsync()) ([Windows.Media.Control.GlobalSystemMediaTransportControlsSessionMediaProperties])
    $status = $info.PlaybackStatus.ToString()
    $expectedStatus = if (-not $process.playback_clock.loaded) { 'Stopped' } elseif ($process.playback_clock.playing) { 'Playing' } else { 'Paused' }
    [pscustomobject]@{
        application_id = $session.SourceAppUserModelId
        playback_status = $status
        playback_rate = $info.PlaybackRate
        position_ms = $timeline.Position.TotalMilliseconds
        duration_ms = ($timeline.EndTime - $timeline.StartTime).TotalMilliseconds
        has_title = [bool]$metadata.Title
        has_artwork = $null -ne $metadata.Thumbnail
        can_play = $info.Controls.IsPlayEnabled
        can_pause = $info.Controls.IsPauseEnabled
        can_seek = $info.Controls.IsPlaybackPositionEnabled
        can_next = $info.Controls.IsNextEnabled
        can_previous = $info.Controls.IsPreviousEnabled
        title_matches_application = $metadata.Title -ceq $state.media_session.title
        status_matches_application = $status -eq $expectedStatus
        # Playing observations are not simultaneous; allow one bounded publish
        # interval plus IPC latency. A paused read must retain its exact frame.
        position_matches_application = [Math]::Abs($timeline.Position.TotalMilliseconds - $process.playback_clock.position_ms) -le $(if ($process.playback_clock.playing) { 1000 } else { 1 })
        rate_matches_application = [Math]::Abs($info.PlaybackRate - $state.playback_rate) -lt 0.000001
        duration_matches_application = [Math]::Abs(($timeline.EndTime - $timeline.StartTime).TotalMilliseconds - $state.duration * 1000) -lt 1
    }
})
if ($sessions.Count -eq 0) { throw 'No matching published Windows media session was found.' }
ConvertTo-Json -InputObject $sessions -Depth 4
if (@($sessions | Where-Object { -not $_.title_matches_application -or -not $_.status_matches_application -or
    -not $_.position_matches_application -or -not $_.rate_matches_application -or -not $_.duration_matches_application }).Count) {
    throw 'Windows session differs from the application snapshot; inspect the comparison flags.'
}
