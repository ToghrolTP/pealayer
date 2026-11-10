# Read-only measurement of neutral foreground ink in real enlarged screenshots.
# Rectangles are image pixels; PixelScale converts the result to capture pixels.
param(
    [Parameter(Mandatory=$true)][string]$InputPath,
    [Parameter(Mandatory=$true)][int[]]$IconRectangle,
    [Parameter(Mandatory=$true)][int[]]$DigitsRectangle,
    [ValidateRange(1,8)][int]$PixelScale=1,
    [ValidateRange(0,255)][int]$InkThreshold=140
)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
$bitmap=[System.Drawing.Bitmap]::new([IO.Path]::GetFullPath($InputPath))
try {
    foreach ($region in @(@{Name='Icon';Rect=$IconRectangle},@{Name='Digits';Rect=$DigitsRectangle})) {
        $rect=$region.Rect
        if ($rect.Count -ne 4 -or $rect[0] -lt 0 -or $rect[1] -lt 0 -or $rect[2] -le 0 -or $rect[3] -le 0 -or
            ([long]$rect[0]+$rect[2]) -gt $bitmap.Width -or ([long]$rect[1]+$rect[3]) -gt $bitmap.Height) {
            throw 'Rectangles must be x,y,width,height inside the image.'
        }
        $top=$bitmap.Height; $bottom=-1
        for ($x=$rect[0];$x -lt ($rect[0]+$rect[2]);$x++) {
            for ($y=$rect[1];$y -lt ($rect[1]+$rect[3]);$y++) {
                $pixel=$bitmap.GetPixel($x,$y)
                if ($pixel.R -gt $InkThreshold -and [Math]::Abs([int]$pixel.R-[int]$pixel.G) -lt 10 -and
                    [Math]::Abs([int]$pixel.R-[int]$pixel.B) -lt 10) {
                    $top=[Math]::Min($top,$y); $bottom=[Math]::Max($bottom,$y)
                }
            }
        }
        if ($bottom -lt 0) { throw "No neutral foreground in $($region.Name) rectangle." }
        [pscustomobject]@{Region=$region.Name;Top=$top;Bottom=$bottom;CapturePixelCenter=($top+$bottom)/(2.0*$PixelScale)}
    }
} finally { $bitmap.Dispose() }
