# Privacy-safe, pixel-preserving crop of an existing real native screenshot.
# Does not capture/control a window, redraw UI, resample, or alter source pixels.
param(
    [Parameter(Mandatory = $true)][string]$InputPath,
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][int[]]$CropRectangle
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$sourcePath = [IO.Path]::GetFullPath($InputPath)
$targetPath = [IO.Path]::GetFullPath($OutputPath)
if ($sourcePath -eq $targetPath) { throw 'Keep the original capture unchanged.' }
if (Test-Path -LiteralPath $targetPath) { throw 'Refusing to overwrite an existing capture.' }
$source = [System.Drawing.Bitmap]::new($sourcePath)
try {
    if ($CropRectangle.Count -ne 4 -or $CropRectangle[0] -lt 0 -or $CropRectangle[1] -lt 0 -or
        $CropRectangle[2] -le 0 -or $CropRectangle[3] -le 0 -or
        ([long]$CropRectangle[0] + $CropRectangle[2]) -gt $source.Width -or
        ([long]$CropRectangle[1] + $CropRectangle[3]) -gt $source.Height) {
        throw 'CropRectangle must be x,y,width,height entirely inside the original capture.'
    }
    $region = [System.Drawing.Rectangle]::new($CropRectangle[0], $CropRectangle[1], $CropRectangle[2], $CropRectangle[3])
    $crop = $source.Clone($region, $source.PixelFormat)
    try { $crop.Save($targetPath, [System.Drawing.Imaging.ImageFormat]::Png) }
    finally { $crop.Dispose() }
} finally { $source.Dispose() }
Get-FileHash -LiteralPath $targetPath -Algorithm SHA256
