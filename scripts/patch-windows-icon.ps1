[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Mandatory)]
    [string]$Executable,
    [Parameter(Mandatory)]
    [string]$Icon,
    [string]$Tool,
    [switch]$NoBackup
)

$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path -LiteralPath $Executable).Path
$ico = (Resolve-Path -LiteralPath $Icon).Path
if ([IO.Path]::GetExtension($ico) -ine '.ico') { throw 'Windows executable resources require an .ico file.' }

function Resolve-ResourceTool([string]$Requested) {
    if ($Requested) { return (Resolve-Path -LiteralPath $Requested).Path }
    foreach ($name in @('rcedit.exe', 'ResourceHacker.exe')) {
        $command = Get-Command $name -ErrorAction SilentlyContinue
        if ($command) { return $command.Source }
    }
    foreach ($candidate in @(
        (Join-Path $env:ProgramFiles 'Pealayer Tools\rcedit.exe'),
        (Join-Path $env:ProgramFiles 'Resource Hacker\ResourceHacker.exe')
    )) {
        if (Test-Path -LiteralPath $candidate) { return $candidate }
    }
    throw 'Neither rcedit.exe nor ResourceHacker.exe was found. Pass -Tool with an installed resource editor.'
}
$resourceTool = Resolve-ResourceTool $Tool
$before = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
$backup = "$exe.before-icon-patch"

if ($PSCmdlet.ShouldProcess($exe, "replace Windows application icon using $resourceTool")) {
    if (-not $NoBackup) { Copy-Item -LiteralPath $exe -Destination $backup -Force }
    if ([IO.Path]::GetFileName($resourceTool) -ieq 'rcedit.exe') {
        & $resourceTool $exe --set-icon $ico
        if ($LASTEXITCODE -ne 0) { throw "rcedit failed with exit code $LASTEXITCODE" }
    } else {
        $temporary = "$exe.icon-patch.tmp.exe"
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
        & $resourceTool -open $exe -save $temporary -action addoverwrite -res $ico -mask 'ICONGROUP,MAINICON,0'
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $temporary)) {
            throw "Resource Hacker failed with exit code $LASTEXITCODE"
        }
        Move-Item -LiteralPath $temporary -Destination $exe -Force
    }
    $after = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
    if ($after -eq $before) { throw 'The executable hash did not change; no icon resource was patched.' }
    $signature = Get-AuthenticodeSignature -LiteralPath $exe
    [pscustomobject]@{
        Executable = $exe
        Icon = $ico
        BeforeSha256 = $before
        AfterSha256 = $after
        Backup = $(if ($NoBackup) { $null } else { $backup })
        SignatureStatus = $signature.Status
        Note = 'Patch before signing: any existing Authenticode signature is invalidated by resource changes.'
    }
}
