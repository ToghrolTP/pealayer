[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Executable,
    [string]$OutputDirectory,
    [string]$AppName,
    [string]$Branding,
    [ValidateSet('dark', 'light')]
    [string]$Theme = 'dark',
    [ValidateSet('en', 'fa')]
    [string[]]$Locale = @('en', 'fa'),
    [ValidateRange(800, 3840)]
    [int]$Width = 1280,
    [ValidateRange(600, 2160)]
    [int]$Height = 800,
    [ValidateRange(1, 30)]
    [int]$StartupTimeoutSeconds = 15
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'This screenshot updater requires Windows.'
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
$artifactProductName = (Get-Item -LiteralPath $resolvedExecutable).VersionInfo.ProductName
$effectiveAppName = if ($AppName) { $AppName.Trim() } elseif ($artifactProductName) { $artifactProductName } else { 'Application' }
$resolvedBranding = if ($Branding) { (Resolve-Path -LiteralPath $Branding -ErrorAction Stop).Path } else { $null }
if (-not $OutputDirectory) {
    $OutputDirectory = Join-Path $repositoryRoot 'docs\screenshots'
}
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$resolvedOutput = (Resolve-Path -LiteralPath $OutputDirectory).Path

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PealayerScreenshotNative {
    [StructLayout(LayoutKind.Sequential)]
    public struct Rect { public int Left, Top, Right, Bottom; }

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);

    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(
        IntPtr hwnd, IntPtr insertAfter, int x, int y, int width, int height, uint flags);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hwnd);

    [DllImport("user32.dll")]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr value);

    [DllImport("user32.dll")]
    public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);

    [DllImport("user32.dll")]
    public static extern bool BringWindowToTop(IntPtr hwnd);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);

    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hwnd, IntPtr deviceContext, uint flags);

    [DllImport("dwmapi.dll")]
    public static extern int DwmGetWindowAttribute(
        IntPtr hwnd, uint attribute, out Rect value, uint valueSize);
}
'@

# PowerShell is DPI-unaware by default. Without this opt-in, user32 virtualizes
# window coordinates while CopyFromScreen consumes physical pixels.
[void][PealayerScreenshotNative]::SetProcessDpiAwarenessContext([IntPtr]::new(-4))
[void][PealayerScreenshotNative]::SetThreadDpiAwarenessContext([IntPtr]::new(-4))

function Test-NearUniformBlack([Drawing.Bitmap]$Bitmap) {
    $minimum = 255
    $maximum = 0
    $stepX = [Math]::Max(1, [Math]::Floor($Bitmap.Width / 32))
    $stepY = [Math]::Max(1, [Math]::Floor($Bitmap.Height / 20))
    for ($y = 0; $y -lt $Bitmap.Height; $y += $stepY) {
        for ($x = 0; $x -lt $Bitmap.Width; $x += $stepX) {
            $pixel = $Bitmap.GetPixel($x, $y)
            $minimum = [Math]::Min($minimum, [Math]::Min($pixel.R, [Math]::Min($pixel.G, $pixel.B)))
            $maximum = [Math]::Max($maximum, [Math]::Max($pixel.R, [Math]::Max($pixel.G, $pixel.B)))
        }
    }
    return $maximum -lt 12 -or ($maximum - $minimum) -lt 3
}

function Wait-MainWindow([Diagnostics.Process]$Process, [int]$TimeoutSeconds) {
    $deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTime]::UtcNow -lt $deadline) {
        if ($Process.HasExited) {
            throw "Pealayer exited before its window was ready (exit $($Process.ExitCode))."
        }
        $Process.Refresh()
        if ($Process.MainWindowHandle -ne [IntPtr]::Zero) {
            return $Process.MainWindowHandle
        }
        Start-Sleep -Milliseconds 100
    }
    throw "Pealayer did not expose a main window within $TimeoutSeconds seconds. Run this script in the signed-in interactive desktop session."
}

function Save-WindowScreenshot([IntPtr]$Handle, [int]$ExpectedProcessId, [string]$Path) {
    [uint32]$windowProcessId = 0
    [void][PealayerScreenshotNative]::GetWindowThreadProcessId($Handle, [ref]$windowProcessId)
    if ($windowProcessId -ne $ExpectedProcessId) {
        throw "Refusing to capture window owned by PID $windowProcessId; expected $ExpectedProcessId."
    }

    $flags = 0x0040 # SWP_SHOWWINDOW
    if (-not [PealayerScreenshotNative]::SetWindowPos($Handle, [IntPtr]::Zero, 32, 32, $Width, $Height, $flags)) {
        throw 'Could not resize the Pealayer window.'
    }
    [void][PealayerScreenshotNative]::BringWindowToTop($Handle)
    [void][PealayerScreenshotNative]::SetForegroundWindow($Handle)
    Start-Sleep -Milliseconds 900

    $rect = New-Object PealayerScreenshotNative+Rect
    if (-not [PealayerScreenshotNative]::GetWindowRect($Handle, [ref]$rect)) {
        throw 'Could not read the Pealayer window bounds.'
    }
    $physicalRect = New-Object PealayerScreenshotNative+Rect
    $dwmFrameBounds = 9 # DWMWA_EXTENDED_FRAME_BOUNDS, always physical pixels
    if ([PealayerScreenshotNative]::DwmGetWindowAttribute(
            $Handle,
            $dwmFrameBounds,
            [ref]$physicalRect,
            [Runtime.InteropServices.Marshal]::SizeOf($physicalRect)
        ) -eq 0) {
        $rect = $physicalRect
    }
    $captureWidth = $rect.Right - $rect.Left
    $captureHeight = $rect.Bottom - $rect.Top
    $bitmap = New-Object Drawing.Bitmap $captureWidth, $captureHeight
    $captureMethod = 'user32.PrintWindow(PW_RENDERFULLCONTENT)'
    try {
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        try {
            $deviceContext = $graphics.GetHdc()
            try {
                if (-not [PealayerScreenshotNative]::PrintWindow($Handle, $deviceContext, 2)) {
                    throw 'PrintWindow did not capture the Pealayer window.'
                }
            } finally {
                $graphics.ReleaseHdc($deviceContext)
            }
            if (Test-NearUniformBlack $bitmap) {
                $graphics.CopyFromScreen(
                    $rect.Left,
                    $rect.Top,
                    0,
                    0,
                    [Drawing.Size]::new($captureWidth, $captureHeight),
                    [Drawing.CopyPixelOperation]::SourceCopy
                )
                $captureMethod = 'System.Drawing.Graphics.CopyFromScreen(window-rect)'
            }
        } finally {
            $graphics.Dispose()
        }
        if (Test-NearUniformBlack $bitmap) {
            throw 'Window capture remained blank after the screen-pixel fallback.'
        }
        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $bitmap.Dispose()
    }
    return [ordered]@{
        width = $captureWidth
        height = $captureHeight
        method = $captureMethod
        process_id = $ExpectedProcessId
    }
}

$sourceCommit = (& git -C $repositoryRoot rev-parse HEAD).Trim()
$executableHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedExecutable).Hash.ToLowerInvariant()
$captures = @()
$captureSession = [Guid]::NewGuid().ToString('N')
$captureProfile = Join-Path ([IO.Path]::GetTempPath()) "pealayer-screenshot-$captureSession"
New-Item -ItemType Directory -Path $captureProfile -Force | Out-Null

for ($localeIndex = 0; $localeIndex -lt $Locale.Count; $localeIndex++) {
    $language = $Locale[$localeIndex]
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = $resolvedExecutable
    $start.WorkingDirectory = Split-Path -Parent $resolvedExecutable
    $start.UseShellExecute = $false
    $start.EnvironmentVariables['APP_LOCALE'] = $language
    $start.EnvironmentVariables['APP_DIRECTION'] = 'auto'
    $start.EnvironmentVariables['APP_THEME'] = $Theme
    $start.EnvironmentVariables['APP_NAME'] = $effectiveAppName
    $start.EnvironmentVariables['PEALAYER_CONFIG_FILE'] = Join-Path $captureProfile "$language-settings.json"
    $start.EnvironmentVariables['PEALAYER_HTTP_PORT'] = (28080 + ($localeIndex * 10)).ToString()
    $start.EnvironmentVariables['PEALAYER_WS_PORT'] = (28081 + ($localeIndex * 10)).ToString()
    $start.EnvironmentVariables['PEALAYER_IPC_PORT'] = (28082 + ($localeIndex * 10)).ToString()
    if ($resolvedBranding) {
        $start.EnvironmentVariables['APPLICATION_BRAND'] = $resolvedBranding
    }

    $process = [Diagnostics.Process]::Start($start)
    try {
        [void]$process.WaitForInputIdle(5000)
        $handle = Wait-MainWindow $process $StartupTimeoutSeconds
        $fileName = "pealayer-$language-$Theme.png"
        $path = Join-Path $resolvedOutput $fileName
        $capture = Save-WindowScreenshot $handle $process.Id $path
        $captures += [ordered]@{
            file = $fileName
            locale = $language
            direction = if ($language -eq 'fa') { 'rtl' } else { 'ltr' }
            theme = $Theme
            width = $capture.width
            height = $capture.height
            sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
            capture_method = $capture.method
            process_id = $capture.process_id
            session = 'signed-in-interactive-desktop'
        }
    } finally {
        if (-not $process.HasExited) {
            [void]$process.CloseMainWindow()
            if (-not $process.WaitForExit(3000)) {
                $process.Kill()
                $process.WaitForExit()
            }
        }
        $process.Dispose()
    }
}

$resolvedTempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$resolvedCaptureProfile = [IO.Path]::GetFullPath($captureProfile)
if (-not $resolvedCaptureProfile.StartsWith($resolvedTempRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to clean screenshot profile outside the temporary directory: $resolvedCaptureProfile"
}
Remove-Item -LiteralPath $resolvedCaptureProfile -Recurse -Force

$manifest = [ordered]@{
    format = 'pealayer-screenshots'
    generated_at_utc = [DateTime]::UtcNow.ToString('o')
    git_commit = $sourceCommit
    executable = Split-Path -Leaf $resolvedExecutable
    executable_sha256 = $executableHash
    application_name = $effectiveAppName
    isolated_profile = $true
    capture_method = 'per-capture'
    captures = $captures
}
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $resolvedOutput 'manifest.json') -Encoding utf8
Write-Host "Updated $($captures.Count) screenshot(s) in $resolvedOutput from $sourceCommit"
