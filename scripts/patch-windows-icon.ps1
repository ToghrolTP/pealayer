[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Mandatory)]
    [string]$Executable,
    [Parameter(Mandatory)]
    [string]$Icon,
    [string]$Tool,
    [string]$UpxTool,
    [switch]$KeepUnpacked,
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

function Test-UpxPacked([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    try {
        $buffer = [byte[]]::new([Math]::Min(16384, [int]$stream.Length))
        $read = $stream.Read($buffer, 0, $buffer.Length)
        $header = [Text.Encoding]::ASCII.GetString($buffer, 0, $read)
        return $header.Contains('UPX0') -and $header.Contains('UPX1')
    } finally {
        $stream.Dispose()
    }
}

function Resolve-UpxTool([string]$Requested, [bool]$Required) {
    if ($Requested) { return (Resolve-Path -LiteralPath $Requested).Path }
    $command = Get-Command 'upx.exe' -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $candidate = Join-Path $env:ProgramFiles 'UPX\upx.exe'
    if (Test-Path -LiteralPath $candidate) { return $candidate }
    if ($Required) {
        throw 'The executable is UPX-packed. Install UPX or pass -UpxTool so it can be safely unpacked before resource editing.'
    }
    return $null
}

$resourceTool = Resolve-ResourceTool $Tool
$wasUpxPacked = Test-UpxPacked $exe
$upx = Resolve-UpxTool $UpxTool $wasUpxPacked
$before = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
$backup = "$exe.before-icon-patch"
$restoreCopy = $(if ($NoBackup) { "$exe.icon-patch.restore.tmp" } else { $backup })

if ($PSCmdlet.ShouldProcess($exe, "replace Windows application icon using $resourceTool")) {
    Copy-Item -LiteralPath $exe -Destination $restoreCopy -Force
    try {
        if ($wasUpxPacked) {
            & $upx -d $exe
            if ($LASTEXITCODE -ne 0) { throw "UPX decompression failed with exit code $LASTEXITCODE" }
        }
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
        if ($wasUpxPacked -and -not $KeepUnpacked) {
            & $upx --best --lzma $exe
            if ($LASTEXITCODE -ne 0) { throw "UPX recompression failed with exit code $LASTEXITCODE" }
            & $upx -t $exe
            if ($LASTEXITCODE -ne 0) { throw "UPX validation failed with exit code $LASTEXITCODE" }
        }
    } catch {
        Copy-Item -LiteralPath $restoreCopy -Destination $exe -Force
        throw
    }
    $after = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
    if ($after -eq $before) { throw 'The executable hash did not change; no icon resource was patched.' }
    if ($NoBackup) { Remove-Item -LiteralPath $restoreCopy -Force }
    $signature = Get-AuthenticodeSignature -LiteralPath $exe
    [pscustomobject]@{
        Executable = $exe
        Icon = $ico
        BeforeSha256 = $before
        AfterSha256 = $after
        Backup = $(if ($NoBackup) { $null } else { $backup })
        WasUpxPacked = $wasUpxPacked
        IsUpxPacked = $(if ($upx) { Test-UpxPacked $exe } else { $false })
        SignatureStatus = $signature.Status
        Note = 'Patch before signing: any existing Authenticode signature is invalidated by resource changes.'
    }
}
