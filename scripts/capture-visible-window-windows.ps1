# Read-only visual verification: never focuses, moves, launches or clicks a window.
param(
    [string]$ProcessName = 'pealayer',
    [Parameter(Mandatory = $true)][string]$OutputPath
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PealayerReadOnlyCapture {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rectangle);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr window);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[void][PealayerReadOnlyCapture]::SetProcessDPIAware()
$windowProcess = @(Get-Process -Name $ProcessName | Where-Object {
    $_.MainWindowHandle -ne 0 -and $_.MainWindowHandle -eq [PealayerReadOnlyCapture]::GetForegroundWindow()
}) | Select-Object -First 1
if (-not $windowProcess) { throw 'Bring the requested application into view first. This script does not change focus.' }
if ([PealayerReadOnlyCapture]::IsIconic($windowProcess.MainWindowHandle)) { throw 'The window is minimized.' }
$rectangle = New-Object PealayerReadOnlyCapture+Rect
if (-not [PealayerReadOnlyCapture]::GetWindowRect($windowProcess.MainWindowHandle, [ref]$rectangle)) { throw 'Cannot read the window rectangle.' }
$width = $rectangle.Right - $rectangle.Left
$height = $rectangle.Bottom - $rectangle.Top
if ($width -le 0 -or $height -le 0) { throw 'Window has no visible extent.' }
$bitmap = New-Object System.Drawing.Bitmap $width, $height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
    $graphics.CopyFromScreen($rectangle.Left, $rectangle.Top, 0, 0, $bitmap.Size)
    $bitmap.Save([IO.Path]::GetFullPath($OutputPath), [System.Drawing.Imaging.ImageFormat]::Png)
    [pscustomobject]@{ ProcessId = $windowProcess.Id; Width = $width; Height = $height; Path = [IO.Path]::GetFullPath($OutputPath) }
} finally {
    $graphics.Dispose()
    $bitmap.Dispose()
}
