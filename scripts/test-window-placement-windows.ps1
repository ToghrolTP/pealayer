# Isolated real-window regression. Never connects hardware, loads media, changes
# production settings, or drives the user's window. Uses the normal product API.
param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [int]$Port=18090,
    [switch]$ObserveOnly,
    [switch]$TestFullPreview,
    [switch]$TestBranding
)
$ErrorActionPreference='Stop'
try { $null=Invoke-RestMethod "http://127.0.0.1:$Port/api/process/authoring" -TimeoutSec 1; throw 'The fixture port is already owned; refusing to touch that process.' }
catch { if ($_.Exception.Message -like '*already owned*') {throw} }
$fixtureDirectory=Join-Path $OutputDirectory ('window-placement-'+[guid]::NewGuid().ToString('N'))
$null=New-Item -ItemType Directory -Path $fixtureDirectory
$fixtureConfig=Join-Path $fixtureDirectory 'config.json'
@{app_name='Pealayer window placement test';auto_connect_hardware=$false;clipboard_url_detection=$false;restore_last_media=$false;last_media_target=$null;
    window_geometry=@{x=100;y=100;width=900;height=650;maximized=$false}} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $fixtureConfig -Encoding utf8
$fixtureMedia=Join-Path $fixtureDirectory 'test-pattern.mp4'
if($TestFullPreview){
    & ffmpeg -v error -f lavfi -i 'testsrc=size=320x180:rate=24' -t 2 -c:v mpeg4 $fixtureMedia
    if($LASTEXITCODE -ne 0){throw 'Synthetic fixture video generation failed.'}
}
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PealayerPlacementFixture {
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd,out Rect rect);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
  [DllImport("user32.dll", SetLastError=true)] public static extern IntPtr SendMessageTimeout(IntPtr hwnd,uint message,IntPtr wp,IntPtr lp,uint flags,uint timeout,out IntPtr result);
}
'@
[void][PealayerPlacementFixture]::SetProcessDPIAware()
function Start-Fixture {
    $process=[System.Diagnostics.Process]::new()
    $process.StartInfo.FileName=$Executable
    $process.StartInfo.WorkingDirectory=Split-Path -Parent $Executable
    $process.StartInfo.Arguments='--pause --workspace nle'
    if($TestFullPreview){$process.StartInfo.Arguments+=' "'+$fixtureMedia+'"'}
    $process.StartInfo.UseShellExecute=$false
    $process.StartInfo.Environment['PEALAYER_CONFIG_FILE']=$fixtureConfig
    $process.StartInfo.Environment['PEALAYER_INSTANCE_ID']='window-placement-test'
    $process.StartInfo.Environment['PEALAYER_PORT']="$Port"
    $process.StartInfo.Environment['PEALAYER_WEB_BIND']='127.0.0.1'
    [void]$process.Start()
    $deadline=[DateTime]::UtcNow.AddSeconds(40)
    do {
        if ($process.HasExited) {throw "Fixture exited: $($process.ExitCode)"}
        try {$state=Invoke-RestMethod "http://127.0.0.1:$Port/api/process/authoring" -TimeoutSec 1;if($state.process_id -eq $process.Id){return $process}} catch {}
        Start-Sleep -Milliseconds 150
    }while([DateTime]::UtcNow -lt $deadline)
    throw 'Isolated fixture API did not become ready.'
}
function Stop-Fixture($process) {
    $authoring=Invoke-RestMethod "http://127.0.0.1:$Port/api/process/authoring"
    if($authoring.process_id -ne $process.Id -or !$authoring.safe_to_replace_native){throw 'Fixture identity/clean-state check failed.'}
    $null=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/command" -Method Post -ContentType application/json -Body '{"command":"quit"}'
    if(!$process.WaitForExit(15000)){throw 'Fixture did not quit gracefully; no force termination requested.'}
    $process.Dispose()
}
$process=$null
try {
    foreach($pass in 1..3){
        $process=Start-Fixture
        Start-Sleep -Milliseconds 500
        $diagnostic=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/taskbar-preview"
        $handle=[IntPtr]$diagnostic.shell_commands.registered_hwnd
        $rect=[PealayerPlacementFixture+Rect]::new()
        if(![PealayerPlacementFixture]::GetClientRect($handle,[ref]$rect)){throw 'Fixture client rectangle unavailable.'}
        $scale=[PealayerPlacementFixture]::GetDpiForWindow($handle)/96.0
        $width=($rect.Right-$rect.Left)/$scale;$height=($rect.Bottom-$rect.Top)/$scale
        "Launch $pass client: $width x $height logical points"
        if(!$ObserveOnly -and ([Math]::Abs($width-900) -gt 3 -or [Math]::Abs($height-650) -gt 3)){throw 'Saved normal placement did not survive restart.'}
        if($TestBranding -and $pass -eq 1){
            $previousIcon=$diagnostic.branding.icon
            $identity=$diagnostic.branding.app_id
            $refreshes=$diagnostic.branding.refreshes
            foreach($preset in 'classic','current','classic','current'){
                $null=Invoke-RestMethod "http://127.0.0.1:$Port/api/config" -Method Post -ContentType application/json -Body (@{app_icon_preset=$preset}|ConvertTo-Json)
                $deadline=[DateTime]::UtcNow.AddSeconds(5)
                do {$diagnostic=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/taskbar-preview";if($diagnostic.branding.icon -ne $previousIcon){break};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline)
                if($diagnostic.branding.icon -eq $previousIcon -or $diagnostic.branding.error){throw 'Live icon change failed.'}
                if($diagnostic.branding.app_id -ne $identity -or $diagnostic.branding.refreshes -ne ++$refreshes){throw 'Icon refresh changed identity or failed to rebuild membership.'}
                if($diagnostic.shell_commands.thumbnail_toolbar.last_error -or $diagnostic.shell_commands.thumbnail_toolbar.added_hwnd -ne $handle.ToInt64()){throw 'Icon refresh lost the media toolbar.'}
                $previousIcon=$diagnostic.branding.icon
                "Live $preset icon applied; stable identity; toolbar retained"
            }
        }
        if($TestFullPreview){
            $deadline=[DateTime]::UtcNow.AddSeconds(10)
            do {$diagnostic=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/taskbar-preview";if($diagnostic.video_only){break};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline)
            if(!$diagnostic.video_only){throw 'Synthetic video did not activate its thumbnail.'}
            $messageResult=[IntPtr]::Zero
            if([PealayerPlacementFixture]::SendMessageTimeout($handle,0x326,[IntPtr]::Zero,[IntPtr]::Zero,2,2000,[ref]$messageResult) -eq [IntPtr]::Zero){throw 'Isolated DWM request timed out.'}
            $deadline=[DateTime]::UtcNow.AddSeconds(10)
            do {$diagnostic=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/taskbar-preview";if($diagnostic.full_preview.captures -gt 0 -and $diagnostic.full_preview.delivered -gt 0){break};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline)
            $diagnostic.full_preview | ConvertTo-Json -Compress | Write-Output
            if($diagnostic.full_preview.captures -lt 1 -or $diagnostic.full_preview.delivered -lt 1 -or $diagnostic.full_preview.error){throw 'Real composite capture/DWM publication failed.'}
            if(!$diagnostic.full_preview.non_uniform){throw 'Composite capture is uniformly blank.'}
            if($diagnostic.full_preview.frame_size[0] -lt 900 -or $diagnostic.full_preview.frame_size[1] -lt 650){throw 'Full preview used a small video bitmap instead of the complete UI.'}
        }
        if($pass -lt 3){
            if($TestFullPreview -and $pass -eq 2){
                $messageResult=[IntPtr]::Zero
                $null=[PealayerPlacementFixture]::SendMessageTimeout($handle,0x112,[IntPtr]0xf020,[IntPtr]::Zero,2,2000,[ref]$messageResult)
            }else{
                $null=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/command" -Method Post -ContentType application/json -Body '{"command":"minimize"}'
            }
            $deadline=[DateTime]::UtcNow.AddSeconds(3)
            while(![PealayerPlacementFixture]::IsIconic($handle) -and [DateTime]::UtcNow -lt $deadline){Start-Sleep -Milliseconds 50}
            if(![PealayerPlacementFixture]::IsIconic($handle)){throw 'Minimize request did not complete.'}
            if($TestFullPreview){
                $before=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/taskbar-preview"
                $messageResult=[IntPtr]::Zero
                $null=[PealayerPlacementFixture]::SendMessageTimeout($handle,0x326,[IntPtr]::Zero,[IntPtr]::Zero,2,2000,[ref]$messageResult)
                $after=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/taskbar-preview"
                if($after.full_preview.delivered -le $before.full_preview.delivered -or $after.full_preview.error){throw 'Minimized Task View cache publication failed.'}
                if($after.full_preview.captures -ne $before.full_preview.captures){throw 'Minimized preview tried reading a zero-size framebuffer.'}
                "Minimized full preview delivered from the last visible UI; no zero-size recapture"
            }
        }
        Stop-Fixture $process
        $process=$null
        if(Test-Path (Join-Path $fixtureDirectory 'workspace-state.ron')){
            $windowLine=Get-Content (Join-Path $fixtureDirectory 'workspace-state.ron') | Where-Object {$_ -match '^\s*"window":'}
            $windowLine | Write-Output
        }
    }
} finally {
    if($process -and !$process.HasExited){
        try {Stop-Fixture $process} catch {Write-Warning "Preserved isolated fixture for inspection: $_"}
    }
    "Fixture evidence retained in $fixtureDirectory"
}
