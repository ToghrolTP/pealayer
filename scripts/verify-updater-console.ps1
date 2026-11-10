[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Invoke-ConsoleProbe([string]$Arguments, [int]$Port) {
    $probe = [System.Diagnostics.Process]::new()
    $started = $false
    try {
        $probe.StartInfo.FileName = $Executable
        $probe.StartInfo.Arguments = $Arguments
        $probe.StartInfo.WorkingDirectory = Split-Path -Parent $Executable
        $probe.StartInfo.UseShellExecute = $false
        $probe.StartInfo.CreateNoWindow = $true
        $probe.StartInfo.RedirectStandardOutput = $true
        $probe.StartInfo.RedirectStandardError = $true
        $probe.StartInfo.Environment['PEALAYER_CONFIG_FILE'] = Join-Path $EvidenceDirectory 'updater-console-isolated.json'
        $probe.StartInfo.Environment['PEALAYER_PORT'] = [string]$Port
        [void]$probe.Start()
        $started = $true
        $output = $probe.StandardOutput.ReadToEndAsync()
        $errors = $probe.StandardError.ReadToEndAsync()
        if (-not $probe.WaitForExit(20000)) { throw 'Console fixture exceeded its 20-second budget.' }
        [pscustomobject]@{ ExitCode = $probe.ExitCode; Output = $output.GetAwaiter().GetResult(); Errors = $errors.GetAwaiter().GetResult() }
    } finally {
        # Only our bounded, hardware-free CLI fixture process can be terminated.
        if ($started -and -not $probe.HasExited) { $probe.Kill(); $probe.WaitForExit() }
        $probe.Dispose()
    }
}

function Invoke-UpdateFixture([string]$Options) {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = $listener.LocalEndpoint.Port
    $digest = 'a' * 64
    $frames = @()
    foreach ($frame in @(
        @('/api/update/from-url', 'downloading', 0),
        @('/api/update/status', 'downloading', 5242880),
        @('/api/update/manifest', 'old', 0),
        @('/api/update/status', 'verifying', 10485760),
        @('/api/update/manifest', 'old', 0),
        @('/api/update/status', 'restarting', 10485760),
        @('/api/update/manifest', 'new', 0)
    )) {
        if ($frame[0] -eq '/api/update/manifest') {
            $body = @{ format='pealayer-update'; service='pealayer'; version='console-fixture'; git_commit='fixture'; git_dirty=$false; platform='windows'; arch='x86_64'; sha256=$(if ($frame[1] -eq 'new') { $digest } else { 'b' * 64 }); size=10485760; artifact_url='/api/update/artifact' }
        } else {
            $body = @{ operation_id='console-fixture'; state=$frame[1]; source='fixture'; bytes_done=$frame[2]; bytes_total=10485760; sha256=$digest; version='console-fixture'; message='Replay fixture; no app or hardware updated'; error=$null }
        }
        $frames += ,@($frame[0], ($body | ConvertTo-Json -Compress))
    }
    $worker = [PowerShell]::Create()
    [void]$worker.AddScript({
        param($Listener, $Frames)
        foreach ($frame in $Frames) {
            $client = $Listener.AcceptTcpClient()
            try {
                if ($frame[0] -eq '/api/update/status' -and $frame[1].Contains('downloading')) { Start-Sleep -Milliseconds 1100 }
                $client.ReceiveTimeout = 15000
                $stream = $client.GetStream()
                $reader = [System.IO.StreamReader]::new($stream, [System.Text.Encoding]::ASCII, $false, 1024, $true)
                $request = $reader.ReadLine()
                if (-not $request.Contains($frame[0])) { throw "Unexpected fixture path: $request" }
                $length = 0
                while ($line = $reader.ReadLine()) {
                    if ($line -match '^Content-Length:\s*(\d+)') { $length = [int]$Matches[1] }
                }
                for ($index = 0; $index -lt $length; $index++) { [void]$reader.Read() }
                $body = [System.Text.Encoding]::UTF8.GetBytes($frame[1])
                $head = [System.Text.Encoding]::ASCII.GetBytes("HTTP/1.1 200 OK`r`nContent-Type: application/json`r`nContent-Length: $($body.Length)`r`nConnection: close`r`n`r`n")
                $stream.Write($head, 0, $head.Length)
                $stream.Write($body, 0, $body.Length)
                $stream.Flush()
            } finally { $client.Dispose() }
        }
    }).AddArgument($listener).AddArgument($frames)
    $running = $worker.BeginInvoke()
    try {
        $result = Invoke-ConsoleProbe "--update-from https://example.invalid/console-fixture.exe --sha256 $digest $Options" $port
        if (-not $running.AsyncWaitHandle.WaitOne(2000)) { throw 'Fixture did not consume expected requests.' }
        [void]$worker.EndInvoke($running)
        if ($worker.HadErrors) { throw ($worker.Streams.Error | Out-String) }
        return $result
    } finally { $listener.Stop(); $worker.Dispose() }
}

if (-not (Test-Path -LiteralPath $EvidenceDirectory -PathType Container)) { throw 'Evidence directory must already exist.' }
$plain = Invoke-UpdateFixture '--no-color'
if ($plain.ExitCode -ne 0 -or $plain.Errors -or $plain.Output.Contains([char]27)) { throw 'Plain console fixture failed.' }
foreach ($text in @('Downloading package', 'Verifying package', 'Restarting peer gracefully', 'Updated build acknowledged by peer')) {
    if (-not $plain.Output.Contains($text)) { throw "Missing console phase: $text" }
}
if ($plain.Output.Split(@('Verifying package'), [System.StringSplitOptions]::None)[1].Contains('%')) { throw 'Stale transfer percentage leaked into verification.' }
$plain.Output | Set-Content -LiteralPath (Join-Path $EvidenceDirectory 'updater-console-plain.txt') -Encoding utf8
$colored = Invoke-UpdateFixture '--color always'
if ($colored.ExitCode -ne 0 -or -not $colored.Output.Contains("$([char]27)[38;5;80m") -or -not $colored.Output.Contains("$([char]27)[38;5;114m")) { throw 'Forced semantic color fixture failed.' }
$colored.Output | Set-Content -LiteralPath (Join-Path $EvidenceDirectory 'updater-console-color.ansi') -Encoding utf8
$json = Invoke-UpdateFixture '--json --color always'
$receipt = $json.Output | ConvertFrom-Json
if ($json.ExitCode -ne 0 -or $json.Errors -or $receipt.state -ne 'completed' -or $json.Output.Contains([char]27)) { throw 'JSON console fixture failed.' }
$closed = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
$closed.Start()
$port = $closed.LocalEndpoint.Port
$closed.Stop()
$failure = Invoke-ConsoleProbe "--update-status 127.0.0.1:$port --json" $port
if ($failure.ExitCode -ne 4 -or $failure.Output -or ($failure.Errors | ConvertFrom-Json).state -ne 'failed') { throw 'JSON failure/exit-code fixture failed.' }
[pscustomobject]@{ Fixture='Loopback replay; no installation or hardware commands'; Plain='passed'; ForcedColor='passed'; Json='passed'; FailureExitCode=$failure.ExitCode; Executable=$Executable } | ConvertTo-Json
