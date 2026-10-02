[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Executable,
    [string]$OutputDirectory,
    [string]$AppName,
    [string]$Branding,
    [string]$HardwareEndpoint,
    [string]$WorkspaceDockLayout,
    [ValidateSet('dark', 'light')]
    [string]$Theme = 'dark',
    [ValidateSet('en', 'fa')]
    [string[]]$Locale = @('en', 'fa'),
    [ValidateSet('main', 'preferences')]
    [string]$Surface = 'main',
    [int]$WindowWidth = 0,
    [int]$WindowHeight = 0,
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
$effectiveWindowWidth = if ($WindowWidth -gt 0) { $WindowWidth } elseif ($Surface -eq 'main') { 1920 } else { 1000 }
$effectiveWindowHeight = if ($WindowHeight -gt 0) { $WindowHeight } elseif ($Surface -eq 'main') { 1080 } else { 900 }
if ($effectiveWindowWidth -lt 800 -or $effectiveWindowWidth -gt 3840) {
    throw 'WindowWidth must be between 800 and 3840 pixels.'
}
if ($effectiveWindowHeight -lt 600 -or $effectiveWindowHeight -gt 2160) {
    throw 'WindowHeight must be between 600 and 2160 pixels.'
}

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class PealayerScreenshotNative {
    public delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr parameter);

    [StructLayout(LayoutKind.Sequential)]
    public struct Rect { public int Left, Top, Right, Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    public struct Point { public int X, Y; }

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);

    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr parameter);

    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hwnd);

    [DllImport("user32.dll")]
    public static extern bool GetClientRect(IntPtr hwnd, out Rect rect);

    [DllImport("user32.dll")]
    public static extern bool ClientToScreen(IntPtr hwnd, ref Point point);

    [DllImport("user32.dll")]
    public static extern bool LogicalToPhysicalPointForPerMonitorDPI(IntPtr hwnd, ref Point point);

    [DllImport("user32.dll")]
    public static extern uint GetDpiForWindow(IntPtr hwnd);

    [DllImport("user32.dll")]
    public static extern int GetSystemMetricsForDpi(int index, uint dpi);

    [DllImport("user32.dll")]
    public static extern bool SetWindowPos(
        IntPtr hwnd, IntPtr insertAfter, int x, int y, int width, int height, uint flags);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hwnd);

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hwnd, int command);

    [DllImport("user32.dll")]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr value);

    [DllImport("user32.dll")]
    public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr value);

    [DllImport("user32.dll")]
    public static extern bool BringWindowToTop(IntPtr hwnd);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);

    [DllImport("user32.dll")]
    public static extern IntPtr GetWindowDC(IntPtr hwnd);

    [DllImport("user32.dll")]
    public static extern int ReleaseDC(IntPtr hwnd, IntPtr deviceContext);

    [DllImport("user32.dll")]
    public static extern bool PrintWindow(IntPtr hwnd, IntPtr deviceContext, uint flags);

    [DllImport("dwmapi.dll")]
    public static extern int DwmGetWindowAttribute(
        IntPtr hwnd, uint attribute, out Rect value, uint valueSize);

    [DllImport("gdi32.dll")]
    public static extern bool BitBlt(
        IntPtr destination, int x, int y, int width, int height,
        IntPtr source, int sourceX, int sourceY, uint operation);

    public static IntPtr FindLargestVisibleWindow(uint expectedProcessId) {
        IntPtr best = IntPtr.Zero;
        long bestArea = 0;
        EnumWindows((hwnd, parameter) => {
            uint processId;
            GetWindowThreadProcessId(hwnd, out processId);
            Rect rect;
            if (processId == expectedProcessId && IsWindowVisible(hwnd) && GetWindowRect(hwnd, out rect)) {
                int width = Math.Max(0, rect.Right - rect.Left);
                int height = Math.Max(0, rect.Bottom - rect.Top);
                long area = (long)width * height;
                if (width >= 320 && height >= 240 && area > bestArea) {
                    best = hwnd;
                    bestArea = area;
                }
            }
            return true;
        }, IntPtr.Zero);
        return best;
    }
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
        $handle = [PealayerScreenshotNative]::FindLargestVisibleWindow([uint32]$Process.Id)
        if ($handle -ne [IntPtr]::Zero) {
            return $handle
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

    [void][PealayerScreenshotNative]::BringWindowToTop($Handle)
    [void][PealayerScreenshotNative]::SetForegroundWindow($Handle)
    Start-Sleep -Milliseconds 1800

    $rect = New-Object PealayerScreenshotNative+Rect
    if (-not [PealayerScreenshotNative]::GetWindowRect($Handle, [ref]$rect)) {
        throw 'Could not read the Pealayer window bounds.'
    }
    $physicalRect = New-Object PealayerScreenshotNative+Rect
    $dwmFrameBounds = 9 # DWMWA_EXTENDED_FRAME_BOUNDS
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
                $windowDc = [PealayerScreenshotNative]::GetWindowDC($Handle)
                if ($windowDc -ne [IntPtr]::Zero) {
                    $targetDc = $graphics.GetHdc()
                    try {
                        $sourceCopyWithLayeredWindows = 0x40CC0020
                        if ([PealayerScreenshotNative]::BitBlt(
                                $targetDc, 0, 0, $captureWidth, $captureHeight,
                                $windowDc, 0, 0, $sourceCopyWithLayeredWindows)) {
                            $captureMethod = 'gdi32.BitBlt(window-dc)'
                        }
                    } finally {
                        $graphics.ReleaseHdc($targetDc)
                        [void][PealayerScreenshotNative]::ReleaseDC($Handle, $windowDc)
                    }
                }
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
                $captureMethod = 'System.Drawing.Graphics.CopyFromScreen(client-frame)'
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
    $start.EnvironmentVariables['PEALAYER_INSTANCE_ID'] = "screenshot-$captureSession-$language"
    $captureConfig = Join-Path $captureProfile "$language-settings.json"
    if ($HardwareEndpoint -or $WorkspaceDockLayout) {
        @{
            hardware_endpoint = if ($HardwareEndpoint) { $HardwareEndpoint.Trim() } else { $null }
            workspace_dock_layout = if ($WorkspaceDockLayout) { $WorkspaceDockLayout } else { $null }
        } |
            ConvertTo-Json |
            Set-Content -LiteralPath $captureConfig -Encoding utf8
    }
    $start.EnvironmentVariables['PEALAYER_CONFIG_FILE'] = $captureConfig
    $start.EnvironmentVariables['PEALAYER_PORT'] = (28080 + ($localeIndex * 10)).ToString()
    if ($Surface -eq 'preferences') {
        $start.ArgumentList.Add('--preferences-helper')
        $start.ArgumentList.Add('0')
        $start.ArgumentList.Add('--preferences-tab')
        $start.ArgumentList.Add('0')
    }
    if ($resolvedBranding) {
        $start.EnvironmentVariables['APPLICATION_BRAND'] = $resolvedBranding
    }

    $process = [Diagnostics.Process]::Start($start)
    try {
        [void]$process.WaitForInputIdle(5000)
        $handle = Wait-MainWindow $process $StartupTimeoutSeconds
        $showWindow = 0x0040
        if (-not [PealayerScreenshotNative]::SetWindowPos(
                $handle,
                [IntPtr]::Zero,
                24,
                24,
                $effectiveWindowWidth,
                $effectiveWindowHeight,
                $showWindow
            )) {
            throw "Could not resize Pealayer to ${effectiveWindowWidth}x${effectiveWindowHeight}."
        }
        Start-Sleep -Milliseconds 500
        $surfaceSuffix = if ($Surface -eq 'main') { '' } else { "-$Surface" }
        $fileName = "pealayer$surfaceSuffix-$language-$Theme.png"
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
    surface = $Surface
    requested_window_size = "${effectiveWindowWidth}x${effectiveWindowHeight}"
    hardware_endpoint = if ($HardwareEndpoint) { $HardwareEndpoint.Trim() } else { $null }
    isolated_profile = $true
    capture_method = 'per-capture'
    captures = $captures
}
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $resolvedOutput 'manifest.json') -Encoding utf8
Write-Host "Updated $($captures.Count) screenshot(s) in $resolvedOutput from $sourceCommit"
