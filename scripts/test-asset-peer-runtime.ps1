# Continue test-asset-import-runtime.ps1 using a deliberately isolated consumer.
param([string]$AuthorityBaseUrl='http://127.0.0.1:18090',[string]$ConsumerBaseUrl='http://127.0.0.1:18092',[Parameter(Mandatory)][string]$FixtureDirectory)
$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
foreach($url in @($AuthorityBaseUrl,$ConsumerBaseUrl)){if(-not ([uri]$url).IsLoopback -or ([uri]$url).Port -eq 8080){throw 'Use only isolated loopback review ports.'}}
$client=Invoke-RestMethod "$ConsumerBaseUrl/api/client/status" -TimeoutSec 5
$config=Invoke-RestMethod "$AuthorityBaseUrl/api/config" -TimeoutSec 5
if($client.role -ne 'client' -or -not $client.connected -or $client.server.TrimEnd('/') -ne $AuthorityBaseUrl.TrimEnd('/') -or $config.app_name -ne 'Pealayer Asset Review' -or $config.auto_connect_hardware){throw 'Only the isolated review fixture pair may be tested.'}
$source=Join-Path $FixtureDirectory 'consumer.wav'
Copy-Item -LiteralPath (Join-Path $FixtureDirectory 'silence.wav') -Destination $source
$body=@{command='import_assets';request=@{paths=@($source);destination=@{surface='library'}}}|ConvertTo-Json -Depth 6 -Compress
$null=Invoke-RestMethod "$ConsumerBaseUrl/api/process/command" -Method Post -ContentType 'application/json' -Body $body -TimeoutSec 8
$limit=[datetime]::UtcNow.AddSeconds(15)
do {
    $config=Invoke-RestMethod "$AuthorityBaseUrl/api/config" -TimeoutSec 5
    $effect=@($config.audio_effects|Where-Object name -eq 'consumer')
    if($effect.Count -eq 1){break}
    Start-Sleep -Milliseconds 100
}while([datetime]::UtcNow -lt $limit)
if($effect.Count -ne 1 -or $effect[0].duration_ms -ne 1000){throw 'Consumer asset did not reach authority.'}
if($effect[0].program.source -eq $source){throw 'Client path forwarded instead of transferring bytes.'}
$state=Invoke-RestMethod "$AuthorityBaseUrl/api/player/status"
if($state.controller_connected -or $state.playing){throw 'Import unexpectedly connected hardware or started playback.'}
Start-Sleep -Milliseconds 1100
$authoring=Invoke-RestMethod "$ConsumerBaseUrl/api/process/authoring" -TimeoutSec 6
if(-not $authoring.safe_to_replace_native){throw 'Acknowledged consumer replica is incorrectly reported as a local draft.'}
[pscustomobject]@{NativeLocalImport='passed';TransferredBytes='passed';AuthoritativeProbeMS=$effect[0].duration_ms;HardwareConnected=$false;PlaybackStarted=$false;ConsumerAuthoring=$authoring.state}
