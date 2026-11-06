[CmdletBinding()]
param([ValidateSet('Both', 'Historical', 'Current')][string]$Version = 'Both')

# Deliberately separate from start-ui-review: this historical installation is
# retained until the user explicitly requests removal, including its edits.
$ErrorActionPreference = 'Stop'
$programRoot = Join-Path $env:LOCALAPPDATA 'Programs\Pealayer'
$reviews = @(
    @{ Kind='Historical'; Folder='pre-craftui-2026-10-10'; Port=18092; Instance='ui-history-october10' },
    @{ Kind='Current'; Folder='current-egui-dock'; Port=18093; Instance='ui-history-current' }
)
$names = @('PEALAYER_CONFIG_FILE', 'PEALAYER_INSTANCE_ID', 'PEALAYER_PORT', 'PEALAYER_WEB_BIND', 'APP_NAME')
foreach ($review in $reviews) {
    if ($Version -ne 'Both' -and $Version -ne $review.Kind) { continue }
    $directory = Join-Path $programRoot ('investigation\' + $review.Folder)
    $configPath = Join-Path $directory 'profile\config.json'
    $config = Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json
    if ($config.auto_connect_hardware -ne $false -or $config.hardware_endpoint -ne 'pccontroller://127.0.0.1:18999' -or
        $config.media_keys_enabled -ne $false -or $config.global_hardware_hotkeys_enabled -ne $false) {
        throw "$($review.Kind) profile must remain hardware-disconnected without global media/hardware shortcuts."
    }
    $existing = Get-NetTCPConnection -State Listen -LocalPort $review.Port -ErrorAction SilentlyContinue
    if ($existing) {
        $live = Invoke-RestMethod "http://127.0.0.1:$($review.Port)/api/config" -TimeoutSec 5
        if ($live.app_name -ne $config.app_name) { throw "Port $($review.Port) belongs to another application." }
        Write-Host "$($review.Kind) comparison is already running on $($review.Port)."
        continue
    }
    $executable = if ($review.Kind -eq 'Historical') {
        Join-Path $directory 'pealayer.exe'
    } else { Join-Path $programRoot 'bin\pealayer.exe' }
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) { throw "Missing comparison executable: $executable" }
    $previous = @{}
    foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
    try {
        $env:PEALAYER_CONFIG_FILE = $configPath
        $env:PEALAYER_INSTANCE_ID = $review.Instance
        $env:PEALAYER_PORT = [string]$review.Port
        $env:PEALAYER_WEB_BIND = '127.0.0.1'
        $env:APP_NAME = $config.app_name
        $process = Start-Process -FilePath $executable -WorkingDirectory (Split-Path -Parent $executable) -ArgumentList '--workspace nle --pause --activate' -PassThru
        Write-Host "$($review.Kind) comparison launched as $($process.Id); API port $($review.Port)."
    } finally {
        foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name, $previous[$name], 'Process') }
    }
}
