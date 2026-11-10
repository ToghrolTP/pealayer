#Requires -Version 7.0
# Real isolated host/API acceptance. Never touches the regular app or hardware.
param(
    [Parameter(Mandatory=$true)][string]$Executable,
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [int]$Port=18192,
    [int]$ExistingProcessId=0,
    [switch]$KeepOpen
)
$ErrorActionPreference='Stop'
$root=[IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs\Pealayer\verification'))
$output=[IO.Path]::GetFullPath($OutputDirectory)
if(-not $output.StartsWith($root+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Use a private directory under Pealayer verification.'}
if(-not $ExistingProcessId){
    try {$null=Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/healthz" -TimeoutSec 1;throw 'Fixture port already owned.'}catch{if($_.Exception.Message -eq 'Fixture port already owned.'){throw}}
}
$null=New-Item -ItemType Directory -Force -Path $output
$configPath=Join-Path $output 'config.json'
if((Test-Path -LiteralPath $configPath) -and -not $ExistingProcessId){throw 'Refusing to overwrite an existing fixture configuration.'}
$config=@{app_name='Pealayer Terminal Review';auto_connect_hardware=$false;clipboard_url_detection=$false;restore_last_media=$false;last_media_target=$null;
    theme='dark';color_palette='studio';web_enabled=$true;web_allow_control=$true;web_allow_configuration=$true;
    window_geometry=@{x=60;y=50;width=1480;height=940;maximized=$false};
    ui=@{terminal=@{enabled=$true;target='portable_shell';allow_remote=$true;allow_system_shell=$false;allow_remote_system_shell=$false}}}
if(-not $ExistingProcessId){[IO.File]::WriteAllText($configPath,($config|ConvertTo-Json -Depth 10),[Text.UTF8Encoding]::new($false))}
$names=@('PEALAYER_CONFIG_FILE','PEALAYER_INSTANCE_ID','PEALAYER_PORT','PEALAYER_WEB_BIND')
$previous=@{};foreach($name in $names){$previous[$name]=[Environment]::GetEnvironmentVariable($name,'Process')}
$process=$null;$owned=[Collections.Generic.List[long]]::new();$key=$null
function Native([string]$method,$params=@{}) {
    $payload=@{jsonrpc='2.0';id=1;method=$method;params=$params}|ConvertTo-Json -Depth 12 -Compress
    $catalog=Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/api/player/commands"
    $pipeName=($catalog.native_ipc.endpoint -split '\\')[-1]
    $pipe=[IO.Pipes.NamedPipeClientStream]::new('.', $pipeName, [IO.Pipes.PipeDirection]::InOut)
    try {
        $pipe.Connect(2000);$pipe.ReadMode=[IO.Pipes.PipeTransmissionMode]::Message
        $bytes=[Text.Encoding]::UTF8.GetBytes($payload+"`n");$pipe.Write($bytes,0,$bytes.Length)
        $buffer=[byte[]]::new(65536);$count=$pipe.Read($buffer,0,$buffer.Length)
        $raw=[Text.Encoding]::UTF8.GetString($buffer,0,$count)
        if(-not $pipe.IsMessageComplete){throw 'Native terminal reply exceeded its bounded buffer.'}
    } finally {$pipe.Dispose()}
    $reply=$raw|ConvertFrom-Json
    if($reply.error){throw $reply.error.message}
    $reply.result
}
function Control($operation) {
    Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/api/terminal/control" -Method Post -ContentType application/json -Headers @{Authorization="Bearer $key"} -Body ($operation|ConvertTo-Json -Compress) -TimeoutSec 8
}
function Denied([string]$path,$body,$headers=@{}) {
    $response=Invoke-WebRequest -NoProxy -SkipHttpErrorCheck "http://127.0.0.1:$Port$path" -Method Post -ContentType application/json -Headers $headers -Body ($body|ConvertTo-Json -Depth 16 -Compress) -TimeoutSec 8
    if($response.StatusCode -notin @(400,403)){throw "Expected denial at $path, got $($response.StatusCode)"}
}
try {
    $env:PEALAYER_CONFIG_FILE=$configPath;$env:PEALAYER_INSTANCE_ID='terminal-host-review';$env:PEALAYER_PORT="$Port";$env:PEALAYER_WEB_BIND='127.0.0.1'
    if($ExistingProcessId){
        $status=Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/api/process/status"
        $live=Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/api/config"
        if($status.process_id -ne $ExistingProcessId -or $live.app_name -ne 'Pealayer Terminal Review' -or $live.auto_connect_hardware -or $status.runtime.executable.sha256 -ne (Get-FileHash -LiteralPath $Executable -Algorithm SHA256).Hash.ToLowerInvariant()){throw 'Existing process is not the exact owned isolated fixture.'}
        $process=Get-Process -Id $ExistingProcessId
    }else{$process=Start-Process -FilePath $Executable -WorkingDirectory (Join-Path $env:LOCALAPPDATA 'Programs\Pealayer\bin') -ArgumentList '--workspace nle --pause' -WindowStyle Hidden -PassThru}
    $ready=$false
    for($n=0;$n -lt 100;$n++){try{$status=Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/api/process/status" -TimeoutSec 1;if($status.process_id -eq $process.Id){$ready=$true;break}}catch{};Start-Sleep -Milliseconds 100}
    if(-not $ready){throw 'Isolated terminal fixture did not start.'}
    Denied '/api/terminal/control' @{operation='snapshot'}
    $pair=Native 'pealayer.terminal.pair';$key=$pair.access_key
    if($key.Length -ne 64){throw 'Native pairing did not return a valid key.'}
    $created=Control @{operation='create';target='portable_shell'};$id=[long]$created.screen.id;$owned.Add($id)
    $ready=$false
    for($n=0;$n -lt 60;$n++){$snapshot=Control @{operation='snapshot'};if(($snapshot.sessions|Where-Object id -eq $id).phase -eq 'ready'){$ready=$true;break};Start-Sleep -Milliseconds 50}
    if(-not $ready){throw 'Embedded shell did not become ready.'}
    $native=Native 'pealayer.terminal.control' @{operation='snapshot'}
    if($native.screen.id -ne $id -or $native.owner_id -ne $snapshot.owner_id){throw 'Native and Web do not share the session owner/ID.'}
    $null=Control @{operation='input';id=$id;data="set ui.terminal.font_size 14`r"}
    $applied=$false
    for($n=0;$n -lt 60;$n++){$live=Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/api/config";if($live.ui.terminal.font_size -eq 14){$applied=$true;break};Start-Sleep -Milliseconds 50}
    if(-not $applied){throw 'Shell setting did not reach real configuration.'}
    $null=Control @{operation='input';id=$id;data="set ui.terminal.font_size 13`rconfigure`r"}
    $table=$false
    for($n=0;$n -lt 60;$n++){$snapshot=Control @{operation='snapshot'};if($snapshot.screen.alternate){$table=$true;break};Start-Sleep -Milliseconds 50}
    if(-not $table){throw 'Upstream settings table did not render.'}
    $other=Control @{operation='create';target='portable_shell'};$otherId=[long]$other.screen.id;$owned.Add($otherId)
    $null=Control @{operation='select';id=$id};$after=Control @{operation='close';id=$otherId};$owned.Remove($otherId)|Out-Null
    if($after.screen.id -ne $id){throw 'Closing another session changed the active identity.'}
    Denied '/api/terminal/control' @{operation='create';target='system_shell'} @{Authorization="Bearer $key"}
    Denied '/api/terminal/control' @{operation='input';id=999999;data='play'} @{Authorization="Bearer $key"}
    Denied '/api/terminal/control' @{operation='snapshot'} @{Authorization="Bearer $key";Origin='https://untrusted.invalid'}
    $live=Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/api/config"
    $live.ui.terminal.allow_system_shell=$true;$live.ui.terminal.allow_remote_system_shell=$true
    Denied '/api/config' @{ui=$live.ui}
    $public=Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/api/ipc" -Method Post -ContentType application/json -Body '{"command":"get_workspace_tools"}'
    if(($public|ConvertTo-Json -Depth 16) -match 'Pealayer settings|ui.terminal.font_size|lines'){throw 'Private screen leaked into general workspace status.'}
    $null=Native 'pealayer.terminal.revoke'
    Denied '/api/terminal/control' @{operation='snapshot'} @{Authorization="Bearer $key"}
    Write-Host 'PASS: native/HTTP shared sessions, real get/set table, selected-ID preservation, default shell denial, security-policy denial, invalid-ID denial, origin isolation, privacy and revocation.'
    [pscustomobject]@{ProcessId=$process.Id;Port=$Port;ConfigPath=$configPath;KeepOpen=[bool]$KeepOpen}
} finally {
    if($process -and -not $KeepOpen) {
        foreach($id in $owned){try{$null=Native 'pealayer.terminal.control' @{operation='close';id=$id}}catch{}}
        try{$null=Native 'pealayer.terminal.revoke'}catch{}
        for($n=0;$n -lt 40;$n++){try{$authoring=Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/api/process/authoring";if($authoring.safe_to_replace_native){break}}catch{};Start-Sleep -Milliseconds 100}
        if($authoring.safe_to_replace_native){$null=Invoke-RestMethod -NoProxy "http://127.0.0.1:$Port/api/ipc" -Method Post -ContentType application/json -Body '{"command":"quit_local"}'}
    }
    foreach($name in $names){[Environment]::SetEnvironmentVariable($name,$previous[$name],'Process')}
}
