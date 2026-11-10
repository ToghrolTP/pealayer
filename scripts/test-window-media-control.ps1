# Complete real-process control matrix. Only the explicitly isolated fixture
# is moved, minimized, played or closed; never use a production API endpoint.
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$UdpProbe,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [int]$Port=18094
)
$ErrorActionPreference='Stop'
if(Test-NetConnection -ComputerName 127.0.0.1 -Port $Port -InformationLevel Quiet -WarningAction SilentlyContinue){throw 'Fixture TCP port is occupied; no process was changed.'}
$null=New-Item -ItemType Directory -Force -Path $OutputDirectory
$fixture=Join-Path $OutputDirectory 'control-transport-fixture'
if(Test-Path -LiteralPath $fixture){throw 'Fixture evidence exists; preserve it and choose a different output directory.'}
$null=New-Item -ItemType Directory -Path $fixture
$configPath=Join-Path $fixture 'config.json'
@{app_name='Pealayer transport fixture';auto_connect_hardware=$false;hardware_endpoint='pccontroller://127.0.0.1:18999';
    clipboard_url_detection=$false;restore_last_media_on_startup=$false;last_media_target=$null;
    media_keys_enabled=$false;global_hardware_hotkeys_enabled=$false;is_muted=$true;web_enabled=$true;
    web_allow_control=$true;web_allow_configuration=$true;web_allow_file_access=$true;web_allow_updates=$false;
    web_port=$Port;web_listen_addresses=@('127.0.0.1');
    binary_udp=@{enabled=$true;listen="127.0.0.1:$Port";discovery=$false;broadcast_events=$false;peers=@{}};
    window_geometry=@{x=80;y=80;width=900;height=650;maximized=$false}
} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $configPath -Encoding utf8
$metadata=Join-Path $fixture 'chapters.txt'
@(';FFMETADATA1','title=Control fixture','[CHAPTER]','TIMEBASE=1/1000','START=0','END=6000','title=Opening','[CHAPTER]','TIMEBASE=1/1000','START=6000','END=12000','title=Second chapter') | Set-Content -LiteralPath $metadata -Encoding utf8
$media=Join-Path $fixture 'control-fixture.mkv'
& ffmpeg -v error -f lavfi -i 'testsrc2=size=320x180:rate=24' -f ffmetadata -i $metadata -map_metadata 1 -map_chapters 1 -c:v mpeg4 -q:v 4 -t 12 -y $media
if($LASTEXITCODE -ne 0){throw 'Synthetic media generation failed.'}
Add-Type -Path (Join-Path $PSScriptRoot 'control-transport-client.cs')
$base="http://127.0.0.1:$Port"
$process=[System.Diagnostics.Process]::new()
$process.StartInfo.FileName=$Executable
$process.StartInfo.WorkingDirectory=Split-Path -Parent $Executable
$process.StartInfo.Arguments='--pause --workspace nle'
$process.StartInfo.UseShellExecute=$false
$process.StartInfo.Environment['PEALAYER_CONFIG_FILE']=$configPath
$process.StartInfo.Environment['PEALAYER_INSTANCE_ID']='control-transport-fixture'
$process.StartInfo.Environment['PEALAYER_PORT']="$Port"
$process.StartInfo.Environment['PEALAYER_WEB_BIND']='127.0.0.1'
$process.StartInfo.Environment['APP_NAME']='Pealayer transport fixture'
$started=$false
$copyClient=$null
$report=[System.Collections.Generic.List[object]]::new()
function Read-Window {Invoke-RestMethod "$base/api/window" -TimeoutSec 3}
function Read-Playback {Invoke-RestMethod "$base/api/player/playback" -TimeoutSec 3}
function Await-State([string]$Kind,[scriptblock]$Condition,[string]$Description) {
    $deadline=[DateTime]::UtcNow.AddSeconds(8)
    do {
        $state=if($Kind -eq 'window'){Read-Window}else{Read-Playback}
        if(& $Condition $state){return $state}
        Start-Sleep -Milliseconds 75
    } while([DateTime]::UtcNow -lt $deadline)
    throw "$Description did not settle: $($state|ConvertTo-Json -Compress -Depth 5)"
}
$script:requestId=0
function Invoke-Control([string]$Transport,[string]$Command,[string]$Method,[hashtable]$Params=@{}) {
    $script:requestId++
    $id=$script:requestId
    $json=@{jsonrpc='2.0';id=$id;method=$Method;params=$Params}|ConvertTo-Json -Compress -Depth 8
    $response=switch($Transport){
        'HTTP' {$plain=@{command=$Command};foreach($key in $Params.Keys){$plain[$key]=$Params[$key]};Invoke-RestMethod "$base/api/player/command" -Method Post -ContentType application/json -Body ($plain|ConvertTo-Json -Compress -Depth 8) -TimeoutSec 5}
        'RPC' {Invoke-RestMethod "$base/api/rpc" -Method Post -ContentType application/json -Body $json -TimeoutSec 5}
        'IPC' {Invoke-RestMethod "$base/api/ipc" -Method Post -ContentType application/json -Body $json -TimeoutSec 5}
        'Pipe' {[PealayerCopyDataClient]::Pipe($script:pipe,$json)|ConvertFrom-Json}
        'CopyData' {$script:copyClient.Call($script:hwnd,$json)|ConvertFrom-Json}
        'TCP' {
            $tcp=[System.Net.Sockets.TcpClient]::new()
            try {
                if(!$tcp.ConnectAsync('127.0.0.1',$Port).Wait(3000)){throw 'TCP connect timed out'}
                $stream=$tcp.GetStream();$stream.ReadTimeout=5000;$stream.WriteTimeout=3000
                $bytes=[Text.Encoding]::UTF8.GetBytes($json+"`n");$stream.Write($bytes,0,$bytes.Length)
                $reader=[IO.StreamReader]::new($stream);$reader.ReadLine()|ConvertFrom-Json
            } finally {$tcp.Dispose()}
        }
        'WebSocket' {
            $socket=[Net.WebSockets.ClientWebSocket]::new();$cancel=[Threading.CancellationTokenSource]::new(5000)
            try {
                $socket.ConnectAsync([uri]"ws://127.0.0.1:$Port/ws",$cancel.Token).GetAwaiter().GetResult()
                $bytes=[Text.Encoding]::UTF8.GetBytes($json)
                $socket.SendAsync([ArraySegment[byte]]::new($bytes),[Net.WebSockets.WebSocketMessageType]::Text,$true,$cancel.Token).GetAwaiter().GetResult()
                do {
                    $message=[IO.MemoryStream]::new();$buffer=[byte[]]::new(16384)
                    do {$received=$socket.ReceiveAsync([ArraySegment[byte]]::new($buffer),$cancel.Token).GetAwaiter().GetResult();$message.Write($buffer,0,$received.Count)}while(!$received.EndOfMessage)
                    $decoded=[Text.Encoding]::UTF8.GetString($message.ToArray())|ConvertFrom-Json;$message.Dispose()
                }while($decoded.id -ne $id)
                $decoded
            } finally {$cancel.Dispose();$socket.Dispose()}
        }
        'UDP' {$text=$json|& $UdpProbe --rpc "127.0.0.1:$Port";if($LASTEXITCODE -ne 0){throw "UDP probe failed: $text"};$text|ConvertFrom-Json}
    }
    if($response.error){throw "$Transport $Method failed: $($response|ConvertTo-Json -Compress -Depth 5)"}
    if($Transport -ne 'HTTP'){
        if($response.id -ne $id){throw "$Transport lost JSON-RPC correlation"}
        return $response.result
    }
    return $response
}
try {
    [void]$process.Start();$started=$true
    $deadline=[DateTime]::UtcNow.AddSeconds(40)
    do {
        if($process.HasExited){throw "Fixture exited: $($process.ExitCode)"}
        try {$identity=Invoke-RestMethod "$base/api/process/authoring" -TimeoutSec 1;if($identity.process_id -eq $process.Id){break}}catch{}
        Start-Sleep -Milliseconds 150
    }while([DateTime]::UtcNow -lt $deadline)
    if($identity.process_id -ne $process.Id){throw 'Fixture identity was not confirmed'}
    $window=Await-State window {$args[0].available -and $args[0].native_handle} 'Native window'
    $script:hwnd=[long]$window.native_handle
    $catalog=Invoke-RestMethod "$base/api/player/commands" -TimeoutSec 3
    $script:pipe=$catalog.native_ipc.endpoint
    $script:copyClient=[PealayerCopyDataClient]::new();$copyClient=$script:copyClient
    foreach($transport in 'HTTP','RPC','IPC','TCP','Pipe','CopyData','WebSocket','UDP'){
        Write-Host "Testing $transport window and media controls..."
        $read=Invoke-Control $transport get_window pealayer.window.get
        if($read.process_id -ne $process.Id){throw 'Window query targeted another process'}
        $width=760+$report.Count*10
        $null=Invoke-Control $transport set_window pealayer.window.set @{x=80;y=80;width=$width;height=520;state='normal'}
        $null=Await-State window {param($s) !$s.minimized -and !$s.maximized -and [Math]::Abs($s.inner_bounds.width-$width)-lt 2 -and [Math]::Abs($s.outer_bounds.x-80)-lt 2} "$transport placement"
        foreach($mode in 'maximized','normal','fullscreen','normal'){
            $null=Invoke-Control $transport set_window pealayer.window.set @{state=$mode}
            $null=Await-State window {param($s) switch($mode){maximized{$s.maximized};fullscreen{$s.fullscreen};normal{!$s.minimized -and !$s.maximized -and !$s.fullscreen}}} "$transport $mode"
        }
        $null=Invoke-Control $transport open pealayer.open @{target=$media}
        $null=Await-State playback {$args[0].loaded -and $args[0].duration_seconds -gt 11} "$transport media load"
        $info=Invoke-Control $transport get_media pealayer.media.get
        if(!$info.information.loaded -or $info.chapters.Count -ne 2 -or $info.media_fps -ne 24){throw "$transport metadata/chapters/FPS are incomplete"}
        $null=Invoke-Control $transport set_volume pealayer.volume.set @{value=37}
        $null=Invoke-Control $transport set_mute pealayer.mute.set @{muted=$false}
        $null=Await-State playback {$args[0].volume -eq 37 -and !$args[0].muted} "$transport volume/mute"
        $null=Invoke-Control $transport toggle_mute pealayer.mute.toggle
        $null=Await-State playback {$args[0].muted} "$transport mute toggle"
        $null=Invoke-Control $transport set_rate pealayer.rate.set @{rate=1.25}
        $null=Await-State playback {$args[0].rate -eq 1.25} "$transport rate"
        $null=Invoke-Control $transport set_window pealayer.window.set @{state='minimized'}
        $null=Await-State window {$args[0].minimized -and $args[0].normal_bounds.width -ge 420} "$transport minimize"
        $null=Invoke-Control $transport play pealayer.play
        $null=Await-State playback {$args[0].playing -and $args[0].position_seconds -gt 0.2} "$transport play while minimized"
        $null=Invoke-Control $transport pause pealayer.pause
        $null=Await-State playback {!$args[0].playing} "$transport pause while minimized"
        $null=Invoke-Control $transport seek_to pealayer.seek_to @{seconds=2.5}
        $null=Await-State playback {[Math]::Abs($args[0].position_seconds-2.5)-lt .09} "$transport exact seek while minimized"
        if(!(Read-Window).minimized){throw 'Media command unexpectedly restored the window'}
        $null=Invoke-Control $transport restore pealayer.window.restore
        $null=Await-State window {!$args[0].minimized -and !$args[0].maximized -and !$args[0].fullscreen} "$transport restore"
        $null=Invoke-Control $transport frame_step pealayer.frame.step @{frames=1}
        $null=Await-State playback {$args[0].position_seconds -gt 2.52 -and !$args[0].playing} "$transport frame step"
        $null=Invoke-Control $transport set_chapter pealayer.chapter.set @{index=1}
        $chapter=Await-State playback {$args[0].position_seconds -ge 5.9} "$transport chapter selection"
        $chapters=Invoke-Control $transport get_chapters pealayer.chapters.get
        if($chapters.chapters.Count -ne 2 -or $chapters.current_chapter_index -ne 1){throw "$transport chapter readback failed"}
        $null=Invoke-Control $transport stop pealayer.stop
        $null=Await-State playback {!$args[0].loaded -and !$args[0].playing} "$transport stop"
        $report.Add([pscustomobject]@{transport=$transport;window='passed';media='passed';minimized_playback='passed';chapters='passed';clipboard_or_hardware_actions=$false})
        $report|ConvertTo-Json -Depth 6|Set-Content -LiteralPath (Join-Path $fixture 'results.json') -Encoding utf8
        Write-Host "$transport passed."
    }
    $report|Format-Table
} finally {
    if($copyClient){$copyClient.Dispose()}
    if($started -and !$process.HasExited){
        $check=Invoke-RestMethod "$base/api/process/authoring" -TimeoutSec 3
        if($check.process_id -ne $process.Id -or !$check.safe_to_replace_native){throw 'Fixture not clean; left running for inspection'}
        $null=Invoke-RestMethod "$base/api/player/command" -Method Post -ContentType application/json -Body '{"command":"quit_local"}' -TimeoutSec 3
        if(!$process.WaitForExit(15000)){throw 'Fixture failed graceful quit; left running without forced termination'}
    }
    $process.Dispose()
}
