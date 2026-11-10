# Run after closing only the retained historical comparison normally.
# No executable replacement, duplicate config, forced exit or registry change.
[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$programRoot=Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Programs\Pealayer'
$directory=[IO.Path]::GetFullPath((Join-Path $programRoot 'investigation\pre-craftui-2026-10-10'))
$executable=Join-Path $directory 'pealayer.exe'
$source=Join-Path $directory 'profile\config.json'
$destination=Join-Path $directory 'pealayer.json'
foreach($path in @($executable,$source,$destination)) {
    if(-not [IO.Path]::GetFullPath($path).StartsWith($directory+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Target escaped the retained comparison directory.'}
}
if((Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash -ne 'e76b61a657fe4ba604c42724ac164632ec1e51689cf7be22005cabdbaf32c345'){throw 'Historical executable identity changed; no configuration moved.'}
$running=@(Get-CimInstance Win32_Process -Filter "Name='pealayer.exe'" | Where-Object {$_.ExecutablePath -and [IO.Path]::GetFullPath($_.ExecutablePath) -eq $executable})
if($running.Count){throw 'Close only the October 10 comparison and its Preferences normally, then rerun. No process was stopped.'}
if(Test-Path -LiteralPath $destination){throw 'Portable profile already exists; preserve it. Nothing moved.'}
$config=Get-Content -LiteralPath $source -Raw|ConvertFrom-Json
if($config.web_port -ne 18092 -or $config.auto_connect_hardware -ne $false -or $config.media_keys_enabled -ne $false -or $config.global_hardware_hotkeys_enabled -ne $false){throw 'Historical profile is not safely isolated; nothing moved.'}
Move-Item -LiteralPath $source -Destination $destination
Write-Host 'Historical settings moved to executable-local pealayer.json. Direct launches now use the dedicated port 18092; no executable changed.'
Start-Process -FilePath $executable -WorkingDirectory $directory -ArgumentList '--workspace nle --pause --activate'
