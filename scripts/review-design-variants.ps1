# Native visual comparison through application APIs and read-only window capture.
# No production profile, pointer/keyboard injection, hardware connection or force quit.
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [Parameter(Mandatory=$true)][ValidatePattern('^[a-z0-9][a-z0-9-]{0,63}$')][string]$Label,
    [ValidateSet('egui_dock','craft_ui')][string]$WorkspaceUi='craft_ui',
    [ValidateSet('compact','film_craft')][string]$MonitorStyle='compact',
    [ValidateSet('standard','film_craft')][string]$TimelineStyle='standard',
    [switch]$Baseline,
    [ValidateSet('dark','light')][string]$Theme='dark',
    [ValidateSet('studio','native')][string]$Palette='studio',
    [ValidateSet('extra_compact','compact','regular','comfortable','spacious')][string]$Density='compact',
    [ValidateSet('en','fa')][string]$Locale='en',
    [int[]]$Widths=@(720,1200,1800),
    [ValidateRange(300,2160)][int]$Height=800,
    [ValidateRange(1024,65535)][int]$Port=18092,
    [switch]$SyntheticMedia,
    [ValidateRange(0.1,11.0)][double]$PositionSeconds=5.25,
    [switch]$KeepOpen
)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'This capture requires Windows.' }
foreach ($width in $Widths) { if ($width -lt 420 -or $width -gt 3840) { throw 'Widths must be 420..3840 logical points.' } }
if (!$Widths.Count) { throw 'At least one width is required.' }
$resolvedExecutable=(Resolve-Path -LiteralPath $Executable).Path
$captureScript=Join-Path $PSScriptRoot 'capture-visible-window-windows.ps1'
$programRoot=Join-Path $env:LOCALAPPDATA 'Programs\Pealayer'
$sessionId='design-review-'+[guid]::NewGuid().ToString('N')
$privateDirectory=Join-Path $programRoot ('verification\'+$sessionId)
$base="http://127.0.0.1:$Port"

# Reject any listener, including a non-Pealayer service, before creating a fixture.
$probe=[Net.Sockets.TcpClient]::new()
try {
    $connect=$probe.ConnectAsync('127.0.0.1',$Port)
    try { [void]$connect.Wait(1000) } catch { }
    if ($probe.Connected) { throw "Port $Port is already occupied; no existing process was changed." }
} finally { $probe.Dispose() }
$null=New-Item -ItemType Directory -Path $privateDirectory
$null=New-Item -ItemType Directory -Path $OutputDirectory -Force
$resolvedOutput=(Resolve-Path -LiteralPath $OutputDirectory).Path
$configPath=Join-Path $privateDirectory 'config.json'
$profile=@{
    app_name='Pealayer design review'; auto_connect_hardware=$false
    clipboard_url_detection=$false; restore_last_media_on_startup=$false
    last_media_target=$null; theme=$Theme; color_palette=$Palette
    accent_color='custom'; custom_accent_color='#6362c7'; language=$Locale
    workspace_ui=$WorkspaceUi; ui_control_density=$Density; osd_timeout_seconds=1
    window_geometry=@{x=60;y=60;width=$Widths[0];height=$Height;maximized=$false}
}
$profile | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $configPath -Encoding utf8
$mediaPath=$null
if ($SyntheticMedia) {
    $mediaPath=Join-Path $privateDirectory 'public-test-pattern.mp4'
    $chapters=Join-Path $privateDirectory 'chapters.ffmetadata'
    @'
;FFMETADATA1
[CHAPTER]
TIMEBASE=1/1000
START=0
END=4000
title=Introduction
[CHAPTER]
TIMEBASE=1/1000
START=4000
END=8000
title=Demonstration
[CHAPTER]
TIMEBASE=1/1000
START=8000
END=12000
title=Finale
'@ | Set-Content -LiteralPath $chapters -Encoding utf8
    & ffmpeg -nostdin -v error -f lavfi -i 'testsrc2=size=640x360:rate=24' -f lavfi -i 'anullsrc=r=48000:cl=stereo' -f ffmetadata -i $chapters -map 0:v -map 1:a -map_metadata 2 -map_chapters 2 -t 12 -c:v mpeg4 -q:v 4 -c:a aac $mediaPath
    if ($LASTEXITCODE -ne 0) { throw 'Synthetic media generation failed.' }
}
function Get-Review([string]$Path) { Invoke-RestMethod "$base$Path" -TimeoutSec 3 }
function Post-Review([string]$Path,$Body) { Invoke-RestMethod "$base$Path" -Method Post -ContentType application/json -Body ($Body | ConvertTo-Json -Depth 12 -Compress) -TimeoutSec 5 }
function Assert-Identity {
    $status=Get-Review '/api/process/status'
    if ($status.process_id -ne $process.Id) { throw 'Fixture API process identity changed; stopping review operations.' }
    return $status
}
function Wait-Review([scriptblock]$Condition,[string]$Description) {
    $deadline=[DateTime]::UtcNow.AddSeconds(30)
    do {
        if ($process.HasExited) { throw "Review process exited ($($process.ExitCode)): $Description" }
        if (& $Condition) { return }
        Start-Sleep -Milliseconds 150
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed out waiting for $Description"
}
$process=[Diagnostics.Process]::new()
$process.StartInfo.FileName=$resolvedExecutable
$process.StartInfo.WorkingDirectory=Split-Path -Parent $resolvedExecutable
$process.StartInfo.UseShellExecute=$false
$process.StartInfo.WindowStyle=[Diagnostics.ProcessWindowStyle]::Hidden
$process.StartInfo.Arguments='--workspace nle --pause'
if ($mediaPath) { $process.StartInfo.Arguments+=' "'+$mediaPath+'"' }
# Override inherited branding only inside the child. Parent environment is untouched.
foreach ($name in @('APPLICATION_BRAND','APP_ICON','APP_NAME','APP_DESCRIPTION','APP_THEME','APP_LOCALE','APP_DIRECTION','PEALAYER_PORTABLE')) {
    $process.StartInfo.Environment.Remove($name) | Out-Null
}
$process.StartInfo.Environment['PEALAYER_CONFIG_FILE']=$configPath
$process.StartInfo.Environment['PEALAYER_INSTANCE_ID']=$sessionId
$process.StartInfo.Environment['PEALAYER_PORT']=[string]$Port
$process.StartInfo.Environment['PEALAYER_WEB_BIND']='127.0.0.1'
$process.StartInfo.Environment['PEALAYER_WEB_ENABLED']='true'
$captures=[Collections.Generic.List[object]]::new()
$launched=$false
try {
    [void]$process.Start(); $launched=$true
    Wait-Review { try { (Get-Review '/api/process/status').process_id -eq $process.Id } catch { $false } } 'the isolated API'
    $identity=Assert-Identity
    if ($SyntheticMedia) {
        Wait-Review {(Get-Review '/api/player/playback').duration_seconds -ge 11.9} 'finite synthetic media'
        $null=Post-Review '/api/rpc' @{jsonrpc='2.0';id=20;method='pealayer.pause';params=@{}}
        $null=Post-Review '/api/rpc' @{jsonrpc='2.0';id=21;method='pealayer.seek_to';params=@{seconds=$PositionSeconds}}
        Wait-Review {
            $playback=Get-Review '/api/player/playback'
            !$playback.playing -and !$playback.seek_pending -and [Math]::Abs($playback.position_seconds-$PositionSeconds) -lt 0.06
        } 'paused nonzero playhead'
    }
    if (!$Baseline) {
        $null=Post-Review '/api/config' @{native_monitor_style=$MonitorStyle;native_timeline_style=$TimelineStyle}
        Wait-Review {
            $live=Get-Review '/api/config'
            $live.native_monitor_style -eq $MonitorStyle -and $live.native_timeline_style -eq $TimelineStyle
        } 'both requested presentation settings'
    }
    $live=Get-Review '/api/config'
    if ($live.auto_connect_hardware -ne $false -or $live.clipboard_url_detection -ne $false -or
        $live.restore_last_media_on_startup -ne $false -or $live.workspace_ui -ne $WorkspaceUi -or
        $live.theme -ne $Theme -or $live.color_palette -ne $Palette) { throw 'Observed fixture settings do not match the requested isolated comparison.' }
    foreach ($width in $Widths) {
        $null=Assert-Identity
        $null=Post-Review '/api/window' @{x=60;y=60;width=$width;height=$Height;state='normal'}
        Wait-Review {
            $window=Get-Review '/api/window'
            $window.process_id -eq $process.Id -and $window.available -and !$window.minimized -and !$window.maximized -and !$window.fullscreen -and
                [Math]::Abs($window.inner_bounds.width-$width) -lt 2 -and [Math]::Abs($window.inner_bounds.height-$Height) -lt 2
        } "observed $width x $Height placement"
        # Let normal IPC feedback expire so it does not obscure the compared UI.
        Start-Sleep -Milliseconds 1500
        $window=Get-Review '/api/window'
        $filename="$Label-$WorkspaceUi-$width.png"
        $output=Join-Path $resolvedOutput $filename
        if (Test-Path -LiteralPath $output) { throw "Capture already exists: $output. Choose another label to preserve prior evidence." }
        $captured=& $captureScript -ExpectedProcessId $process.Id -WindowHandle ([long]$window.native_handle) -OutputPath $output -RenderOccludedWindow
        $captures.Add(@{
            image=$filename; sha256=(Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash.ToLowerInvariant()
            logical_width=$width; logical_height=$Height; pixels_per_point=$window.pixels_per_point
            pixel_width=$captured.Width; pixel_height=$captured.Height; method='PrintWindow-PW_RENDERFULLCONTENT'
            inspected=$false
            playback= $(if($SyntheticMedia){Get-Review '/api/player/playback'}else{$null})
        })
        Write-Host "Captured $filename; inspect the image before claiming visual acceptance."
    }
    $manifest=@{
        format='pealayer-native-design-review'; label=$Label; captured_at_utc=[DateTime]::UtcNow.ToString('o')
        source_commit=$identity.git_commit; source_dirty=$identity.git_dirty
        executable_sha256=(Get-FileHash -LiteralPath $resolvedExecutable -Algorithm SHA256).Hash.ToLowerInvariant()
        workspace_ui=$WorkspaceUi; baseline=[bool]$Baseline
        monitor_style= $(if ($Baseline) {'pre-change'} else {$MonitorStyle})
        timeline_style= $(if ($Baseline) {'pre-change'} else {$TimelineStyle})
        theme=$Theme; palette=$Palette; accent='#6362c7'; density=$Density; locale=$Locale
        media= $(if ($SyntheticMedia) {'Generated 12-second 24-fps test pattern and silent audio'} else {'None'})
        hardware_connected=$false; acceptance='Captured only; human inspection and interaction evidence remain separate'
        captures=$captures.ToArray()
    }
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $resolvedOutput "$Label-$WorkspaceUi-manifest.json") -Encoding utf8
} finally {
    if ($launched -and !$process.HasExited -and !$KeepOpen) {
        try {
            $null=Assert-Identity
            $authoring=Get-Review '/api/process/authoring'
            if ($authoring.process_id -ne $process.Id -or !$authoring.safe_to_replace_native) { throw 'Review draft state is not clean; leaving the process open.' }
            $null=Post-Review '/api/rpc' @{jsonrpc='2.0';id=1;method='pealayer.process.quit';params=@{}}
            if (!$process.WaitForExit(15000)) { throw 'Review did not exit gracefully; no force termination performed.' }
        } catch { Write-Warning $_.Exception.Message }
    }
    if ($launched -and !$process.HasExited) { Write-Host "Review remains open: PID $($process.Id), API $base, private profile $configPath" }
    $process.Dispose()
}
