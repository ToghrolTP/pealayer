# Isolated real-window regression. Never connects hardware, loads media, changes
# production settings, or drives the user's window. Uses the normal product API.
param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [int]$Port=18090,
    [switch]$ObserveOnly,
    [switch]$TestFullPreview,
    [switch]$TestBranding,
    [switch]$TestBrandingApi,
    # Requires a visible primary Windows taskbar. Only matching icon crops are
    # retained; no desktop input or private production media is captured.
    [switch]$TestDisplayedBranding
)
$ErrorActionPreference='Stop'
if($TestDisplayedBranding){$TestBranding=$true;. (Join-Path $PSScriptRoot 'taskbar-artwork-fixture.ps1')}
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
  private delegate bool EnumProc(IntPtr hwnd,IntPtr parameter);
  [DllImport("user32.dll")] private static extern bool EnumWindows(EnumProc callback,IntPtr parameter);
  [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint processId);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] private static extern int GetWindowText(IntPtr hwnd,System.Text.StringBuilder text,int count);
  public static IntPtr TitledWindow(uint processId,string title) {
    IntPtr found=IntPtr.Zero;
    EnumWindows((hwnd,parameter)=>{uint owner;GetWindowThreadProcessId(hwnd,out owner);
      if(owner==processId){var text=new System.Text.StringBuilder(512);GetWindowText(hwnd,text,text.Capacity);
        if(text.ToString()==title){found=hwnd;return false;}}
      return true;},IntPtr.Zero);
    return found;
  }
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
        if($TestBrandingApi -and $pass -eq 1){
            $base="http://127.0.0.1:$Port"
            $original=(Invoke-RestMethod "$base/api/branding").configured
            $null=Invoke-RestMethod "$base/api/player/command" -Method Post -ContentType application/json -Body '{"command":"navigate_preferences","section":"appearance","scroll_y":0}'
            $deadline=[DateTime]::UtcNow.AddSeconds(10)
            do {$helper=Get-CimInstance Win32_Process -Filter "ParentProcessId=$($process.Id)" | Where-Object {$_.CommandLine -like '*--preferences-helper*'};if($helper){break};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline)
            if(@($helper).Count -ne 1){throw 'Isolated native Preferences helper did not open.'}
            function Wait-Branding([scriptblock]$condition){
                $deadline=[DateTime]::UtcNow.AddSeconds(8)
                do {
                    $value=Invoke-RestMethod "$base/api/branding"
                    if(&$condition $value){return $value}
                    Start-Sleep -Milliseconds 80
                }while([DateTime]::UtcNow -lt $deadline)
                throw 'Branding API application timed out.'
            }
            $null=Invoke-RestMethod "$base/api/branding" -Method Post -ContentType application/json -Body '{"values":{"app_name":"Temporary cinema","app_description":"Live session","app_icon_preset":"classic"}}'
            $null=Wait-Branding {param($value) $value.effective.app_name -eq 'Temporary cinema' -and $value.effective.app_description -eq 'Live session' -and $value.temporary.app_icon_preset -eq 'classic'}
            $deadline=[DateTime]::UtcNow.AddSeconds(8)
            do {$helperWindow=[PealayerPlacementFixture]::TitledWindow($helper.ProcessId,'Preferences — Temporary cinema');if($helperWindow -ne [IntPtr]::Zero){break};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline)
            if($helperWindow -eq [IntPtr]::Zero){throw 'Open Preferences did not receive live temporary branding.'}
            $runtime=Invoke-RestMethod "$base/api/runtime/config"
            if($runtime.appName -ne 'Temporary cinema' -or $runtime.appDescription -ne 'Live session'){throw 'Web runtime metadata did not follow temporary branding.'}
            $null=Invoke-RestMethod "$base/api/player/command" -Method Post -ContentType application/json -Body '{"command":"set_volume","value":37}'
            $deadline=[DateTime]::UtcNow.AddSeconds(8)
            do {$disk=Get-Content -LiteralPath $fixtureConfig -Raw | ConvertFrom-Json;if($disk.volume -eq 37){break};Start-Sleep -Milliseconds 80}while([DateTime]::UtcNow -lt $deadline)
            if($disk.volume -ne 37 -or $disk.app_name -ne $original.app_name -or $disk.app_description -ne $original.app_description -or $disk.app_icon_preset -ne $original.app_icon_preset){throw 'Unrelated autosave leaked temporary branding or did not finish.'}
            $null=Invoke-RestMethod "$base/api/branding" -Method Post -ContentType application/json -Body '{"values":{"app_name":"Saved cinema","app_description":"Saved tagline"},"persist":true}'
            $null=Wait-Branding {param($value) $value.configured.app_name -eq 'Saved cinema' -and $value.effective.app_description -eq 'Saved tagline' -and $value.temporary.app_icon_preset -eq 'classic'}
            $disk=Get-Content -LiteralPath $fixtureConfig -Raw | ConvertFrom-Json
            if($disk.app_name -ne 'Saved cinema' -or $disk.app_description -ne 'Saved tagline' -or $disk.app_icon_preset -ne $original.app_icon_preset -or $disk.volume -ne 37){throw 'Selective branding persistence changed an unrelated field.'}
            $null=Invoke-RestMethod "$base/api/branding" -Method Delete
            $null=Wait-Branding {param($value) @($value.temporary.PSObject.Properties).Count -eq 0 -and $value.effective.app_name -eq 'Saved cinema' -and $value.effective.app_icon_preset -eq $original.app_icon_preset}
            $null=Invoke-RestMethod "$base/api/branding" -Method Post -ContentType application/json -Body (@{values=$original;persist=$true}|ConvertTo-Json -Depth 5)
            $null=Wait-Branding {param($value) $value.configured.app_name -eq $original.app_name}
            $deadline=[DateTime]::UtcNow.AddSeconds(8)
            do {$helperWindow=[PealayerPlacementFixture]::TitledWindow($helper.ProcessId,"Preferences — $($original.app_name)");if($helperWindow -ne [IntPtr]::Zero){break};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline)
            if($helperWindow -eq [IntPtr]::Zero){throw 'Preferences title did not restore after branding reset.'}
            $messageResult=[IntPtr]::Zero
            $null=[PealayerPlacementFixture]::SendMessageTimeout($helperWindow,0x10,[IntPtr]::Zero,[IntPtr]::Zero,2,2000,[ref]$messageResult)
            $helperProcess=Get-Process -Id $helper.ProcessId -ErrorAction SilentlyContinue
            if($helperProcess -and !$helperProcess.WaitForExit(8000)){throw 'Clean isolated Preferences did not close normally.'}
            'Branding API: temporary overlay survives autosave without leaking; selective persistence and reset passed'
        }
        if($TestBranding -and $pass -eq 1){
            # The API reset above can still have a coalesced Explorer refresh
            # pending. Establish this phase's baseline only after it finishes.
            $deadline=[DateTime]::UtcNow.AddSeconds(5)
            do {$diagnostic=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/taskbar-preview";if(!$diagnostic.branding.refresh_pending){break};Start-Sleep -Milliseconds 80}while([DateTime]::UtcNow -lt $deadline)
            if($diagnostic.branding.refresh_pending -or $diagnostic.branding.error){throw 'Previous branding phase did not settle.'}
            $artworkBox=$null
            $artworkPass=0
            $previousIcon=$diagnostic.branding.icon
            $identity=$diagnostic.branding.app_id
            $refreshes=$diagnostic.branding.refreshes
            foreach($preset in 'classic','current','classic','current'){
                $null=Invoke-RestMethod "http://127.0.0.1:$Port/api/config" -Method Post -ContentType application/json -Body (@{app_icon_preset=$preset}|ConvertTo-Json)
                $deadline=[DateTime]::UtcNow.AddSeconds(5)
                do {$diagnostic=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/taskbar-preview";if($diagnostic.branding.icon -ne $previousIcon -and !$diagnostic.branding.refresh_pending -and !$diagnostic.shell_commands.thumbnail_toolbar.last_error -and $diagnostic.shell_commands.thumbnail_toolbar.added_hwnd -eq $handle.ToInt64()){break};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline)
                if($diagnostic.branding.icon -eq $previousIcon -or $diagnostic.branding.error){throw 'Live icon change failed.'}
                if($diagnostic.branding.app_id -ne $identity -or $diagnostic.branding.refreshes -ne ++$refreshes){throw 'Icon refresh changed identity or failed to rebuild membership.'}
                if($diagnostic.shell_commands.thumbnail_toolbar.last_error -or $diagnostic.shell_commands.thumbnail_toolbar.added_hwnd -ne $handle.ToInt64()){throw "Icon refresh lost the media toolbar: $($diagnostic.shell_commands.thumbnail_toolbar | ConvertTo-Json -Compress)"}
                $previousIcon=$diagnostic.branding.icon
                "Live $preset icon metadata applied; stable identity; toolbar retained (not visual proof)"
                if($TestDisplayedBranding){
                    Start-Sleep -Milliseconds 800
                    $assets=Join-Path $PSScriptRoot '..\assets'
                    $expected=if($preset -eq 'classic'){Join-Path $assets 'icons\classic.ico'}else{Join-Path $assets 'icon.ico'}
                    $alternate=if($preset -eq 'classic'){Join-Path $assets 'icon.ico'}else{Join-Path $assets 'icons\classic.ico'}
                    $artworkPass++
                    $artworkBox=Assert-TaskbarArtwork -IconPath $expected -OtherIconPath $alternate -Box $artworkBox -OutputPath (Join-Path $fixtureDirectory "taskbar-$artworkPass-$preset.png")
                }
            }
            if($TestDisplayedBranding){
                $completed=$diagnostic.branding.refreshes
                Start-Sleep -Milliseconds 500
                $idle=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/taskbar-preview"
                if($idle.branding.refresh_pending -or $idle.branding.refreshes -ne $completed){throw 'Branding refresh timer did not stop after completion.'}
                'Completed branding refresh remains idle; no recurring shell refresh'
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
                if([PealayerPlacementFixture]::SendMessageTimeout($handle,0x112,[IntPtr]0xf020,[IntPtr]::Zero,2,2000,[ref]$messageResult) -eq [IntPtr]::Zero){throw 'Native system-minimize message timed out.'}
            }else{
                $null=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/command" -Method Post -ContentType application/json -Body '{"command":"minimize"}'
            }
            $deadline=[DateTime]::UtcNow.AddSeconds(3)
            while(![PealayerPlacementFixture]::IsIconic($handle) -and [DateTime]::UtcNow -lt $deadline){Start-Sleep -Milliseconds 50}
            if(![PealayerPlacementFixture]::IsIconic($handle)){
                $state=Invoke-RestMethod "http://127.0.0.1:$Port/api/player/taskbar-preview"
                $state | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $fixtureDirectory "minimize-failure-$pass.json")
                throw "Minimize request did not complete on pass $pass; diagnostics retained."
            }
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
