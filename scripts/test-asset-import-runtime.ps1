# Only the deliberately isolated, disconnected review app may receive these edits.
param([string]$BaseUrl='http://127.0.0.1:18090',[Parameter(Mandatory)][string]$FixtureDirectory,[switch]$LeaveReviewOpen)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$origin=[uri]$BaseUrl
if(-not $origin.IsLoopback -or $origin.Port -eq 8080){throw 'Use the isolated review port, not the regular application.'}
function Get-Api([string]$Path){Invoke-RestMethod "$BaseUrl$Path" -TimeoutSec 10}
function Post-Api([string]$Path,$Value){Invoke-RestMethod "$BaseUrl$Path" -Method Post -ContentType 'application/json' -Body ($Value|ConvertTo-Json -Depth 100 -Compress) -TimeoutSec 15}
function Send-Command($Value){$null=Post-Api '/api/player/command' $Value}
function Wait-Status([scriptblock]$Condition){
    $limit=[datetime]::UtcNow.AddSeconds(15)
    do{$state=Get-Api '/api/player/status';if(& $Condition $state){return $state};Start-Sleep -Milliseconds 100}while([datetime]::UtcNow -lt $limit)
    throw 'Isolated runtime did not reach the expected asset state.'
}
$config=Get-Api '/api/config'
$initial=Get-Api '/api/player/status'
if($config.app_name -ne 'Pealayer Asset Review' -or $config.auto_connect_hardware -ne $false -or $initial.controller_connected -or $initial.current_video -or @($initial.cues).Count){throw 'A fresh, empty, hardware-disconnected Pealayer Asset Review profile is required.'}
$null=New-Item -ItemType Directory -Path $FixtureDirectory -Force
$wav=Join-Path $FixtureDirectory 'silence.wav'
$stream=[IO.MemoryStream]::new();$writer=[IO.BinaryWriter]::new($stream)
try{
    $writer.Write([Text.Encoding]::ASCII.GetBytes('RIFF'));$writer.Write([int]16036);$writer.Write([Text.Encoding]::ASCII.GetBytes('WAVEfmt '));$writer.Write([int]16);$writer.Write([int16]1);$writer.Write([int16]1);$writer.Write([int]8000);$writer.Write([int]16000);$writer.Write([int16]2);$writer.Write([int16]16);$writer.Write([Text.Encoding]::ASCII.GetBytes('data'));$writer.Write([int]16000);$writer.Write([byte[]]::new(16000));$writer.Flush();[IO.File]::WriteAllBytes($wav,$stream.ToArray())
}finally{$writer.Dispose();$stream.Dispose()}
$batch=[guid]::NewGuid().ToString()
$begin=Post-Api '/api/assets/upload/begin' @{batch_id=$batch;name='silence.wav';size=(Get-Item -LiteralPath $wav).Length}
$owner=$begin.owner_id
$null=Invoke-RestMethod "$BaseUrl/api/assets/upload/chunk?id=$($begin.id)&offset=0&owner_id=$owner" -Method Put -InFile $wav -ContentType 'application/octet-stream' -TimeoutSec 10
$receipt=Post-Api "/api/assets/upload/finish?id=$($begin.id)&owner_id=$owner" @{}
$null=Post-Api "/api/assets/upload/retain?owner_id=$owner" @{paths=@($receipt.path)}
Send-Command @{command='import_assets';request=@{owner_id=$owner;paths=@($receipt.path);destination=@{surface='timeline';start_time_ms=10429;track_key='controller-effect:audio'}}}
$state=Wait-Status {param($s) @($s.cues).Count -eq 1 -and -not $s.asset_imports.busy}
if($state.cues[0].start_time_ms -ne 10429 -or $state.cues[0].duration_ms -lt 900 -or $state.playback_time -ne 0){throw 'Audio placement/probe changed exact time or playhead.'}
$session=Get-Api '/api/peer/session'
$fragment=Join-Path $FixtureDirectory 'fragment.json'
[IO.File]::WriteAllText($fragment,($session.timeline.timeline|ConvertTo-Json -Depth 100))
Send-Command @{command='import_assets';request=@{paths=@($fragment);destination=@{surface='timeline';start_time_ms=20000}}}
$state=Wait-Status {param($s) @($s.asset_imports.reviews).Count -eq 1}
if($state.playback_time -ne 0 -or @($state.cues).Count -ne 1){throw 'Opening review mutated timeline or playback.'}
$review=$state.asset_imports.reviews[0]
Send-Command @{command='review_asset';id=$review.id;accept=$true}
$state=Wait-Status {param($s) @($s.cues).Count -eq 2}
if(@($state.cues|Where-Object start_time_ms -eq 30429).Count -ne 1 -or $state.playback_time -ne 0){throw 'Timeline fragment did not retain exact relative offsets.'}
# A mixed movie/subtitle drop uses libmpv's actual FileLoaded and track catalog.
$movie=Join-Path $FixtureDirectory 'black.mkv';$sub=Join-Path $FixtureDirectory 'captions.srt'
& ffmpeg -hide_banner -loglevel error -y -f lavfi -i 'color=c=black:s=64x36:r=2:d=1' -c:v ffv1 $movie
if($LASTEXITCODE -ne 0){throw 'Could not generate isolated movie fixture.'}
[IO.File]::WriteAllText($sub,"1`n00:00:00,000 --> 00:00:00,900`nAsset import fixture`n")
Send-Command @{command='import_assets';request=@{paths=@($sub,$movie);destination=@{surface='player'}}}
$null=Wait-Status {param($s) $s.pending_timeline_change}
# These are this isolated test's own cues, exported above, never user edits.
Send-Command @{command='resolve_timeline_close';decision='discard'}
$state=Wait-Status {param($s) @($s.media_tracks|Where-Object kind -eq 'subtitle').Count -gt 0 -and -not $s.asset_imports.busy}
Send-Command @{command='pause'}
if($state.current_video -ne $movie){throw 'Subtitle attached to the wrong media.'}
$scene=Join-Path $FixtureDirectory 'scene.json'
[IO.File]::WriteAllText($scene,(@{kind='pealayer-scene';data=@{name='Cinema scene';board_identity='';channels=@{'relay.5'=@{type='boolean';on=$true};'pwm.0'=@{type='pwm';percent_milli=35000}};unsampled=@('relay.6')}}|ConvertTo-Json -Depth 20))
Send-Command @{command='import_assets';request=@{paths=@($scene);destination=@{surface='timeline';start_time_ms=10429}}}
$state=Wait-Status {param($s) @($s.asset_imports.reviews).Count -eq 1}
if($state.asset_imports.reviews[0].summary -notcontains 'pwm.0 · 35.000%'){throw 'Scene review values are not authoritative.'}
if(-not $LeaveReviewOpen){Send-Command @{command='review_asset';id=$state.asset_imports.reviews[0].id;accept=$false}}
[pscustomobject]@{Upload='passed';AudioProbe='passed';ExactPlacement='10429 ms';AppendedCue='30429 ms';MovieAndSubtitle='passed';SceneReview='passed';HardwareConnected=$state.controller_connected;ReviewLeftOpen=[bool]$LeaveReviewOpen}
