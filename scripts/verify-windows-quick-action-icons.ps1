[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$resolvedExecutable = (Resolve-Path -LiteralPath $Executable).Path
$definitions = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot '..\src\platform\windows_quick_actions.rs')
$tasks = [regex]::Matches($definitions, 'title: "([^"]+)".*?icon_resource_id: (\d+)', [System.Text.RegularExpressions.RegexOptions]::Singleline)
if ($tasks.Count -ne 7) { throw 'Expected seven shared Windows quick-action definitions.' }

# Read-only PE resource verification; no window capture or UI automation.
Add-Type -AssemblyName System.Drawing
if (-not ('PealayerShellIconResources' -as [type])) {
    $drawingReferences = if ($PSVersionTable.PSEdition -eq 'Core') {
        @('System.Drawing.Common', 'System.Drawing.Primitives')
    } else { @('System.Drawing') }
    foreach ($assemblyName in @('System.Private.Windows.GdiPlus', 'System.Private.Windows.Core')) {
        $assemblyPath = Join-Path $PSHOME ($assemblyName + '.dll')
        if (Test-Path -LiteralPath $assemblyPath) { $drawingReferences += $assemblyPath }
    }
    Add-Type -ReferencedAssemblies $drawingReferences -TypeDefinition @'
using System;
using System.Drawing;
using System.Runtime.InteropServices;
public static class PealayerShellIconResources {
    [DllImport("shell32.dll", CharSet=CharSet.Unicode)]
    private static extern uint ExtractIconEx(string path, int index, out IntPtr large, out IntPtr small, uint count);
    [DllImport("user32.dll")] private static extern bool DestroyIcon(IntPtr icon);
    public static Bitmap Read(string path, int id) {
        IntPtr large, small;
        // Index -1 is the shell's special icon-count query on some versions.
        // The branded application icon is always the first group; tasks use
        // explicit IDs >= 101, which cannot collide with that special value.
        uint count = ExtractIconEx(path, id == 1 ? 0 : -id, out large, out small, 1);
        try {
            if (count == 0 || count == UInt32.MaxValue || large == IntPtr.Zero || small == IntPtr.Zero)
                throw new InvalidOperationException("Cannot extract shell icon resource " + id + " (count=" + count + ")");
            using (Icon icon = Icon.FromHandle(large)) { return icon.ToBitmap(); }
        } finally {
            if (large != IntPtr.Zero) DestroyIcon(large);
            if (small != IntPtr.Zero) DestroyIcon(small);
        }
    }
}
'@
}
$seen = [System.Collections.Generic.HashSet[string]]::new()
$bitmaps = [System.Collections.Generic.List[System.Drawing.Bitmap]]::new()
$appBitmap = [PealayerShellIconResources]::Read($resolvedExecutable, 1)
$bitmaps.Add($appBitmap)
$labels = [System.Collections.Generic.List[string]]::new()
$labels.Add('Pealayer')
try {
    foreach ($task in $tasks) {
        $id = [int]$task.Groups[2].Value
        $bitmap = [PealayerShellIconResources]::Read($resolvedExecutable, $id)
        $bitmaps.Add($bitmap)
        $labels.Add($task.Groups[1].Value)
        $pixels = [System.Collections.Generic.List[byte]]::new()
        $visible = 0
        for ($y = 0; $y -lt $bitmap.Height; $y++) {
            for ($x = 0; $x -lt $bitmap.Width; $x++) {
                $pixel = $bitmap.GetPixel($x, $y)
                $pixels.AddRange([BitConverter]::GetBytes($pixel.ToArgb()))
                if ($pixel.A -gt 0) { $visible++ }
            }
        }
        if ($visible -eq 0) { throw "Empty shell icon resource $id" }
        $sha = [System.Security.Cryptography.SHA256]::Create()
        try { $hash = [BitConverter]::ToString($sha.ComputeHash($pixels.ToArray())).Replace('-', '') } finally { $sha.Dispose() }
        if (-not $seen.Add($hash)) { throw "Duplicate shell icon resource $id" }
        Write-Output ("Verified {0}: resource {1}, {2}x{3}" -f $labels[$labels.Count - 1], $id, $bitmap.Width, $bitmap.Height)
    }
    if ($OutputDirectory) {
        New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
        $atlas = [System.Drawing.Bitmap]::new(1280, 176)
        $graphics = [System.Drawing.Graphics]::FromImage($atlas)
        $font = [System.Drawing.Font]::new('Segoe UI', 10)
        $brushes = @([System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(38,38,42)), [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(246,246,248)))
        $textBrushes = @([System.Drawing.SolidBrush]::new([System.Drawing.Color]::WhiteSmoke), [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(38,38,42)))
        try {
            for ($row = 0; $row -lt 2; $row++) {
                $graphics.FillRectangle($brushes[$row], 0, $row * 88, 1280, 88)
                for ($index = 0; $index -lt $bitmaps.Count; $index++) {
                    $graphics.DrawImageUnscaled($bitmaps[$index], $index * 160 + 64, $row * 88 + 12)
                    $graphics.DrawString($labels[$index], $font, $textBrushes[$row], [single]($index * 160 + 12), [single]($row * 88 + 56))
                }
            }
            $atlas.Save((Join-Path $OutputDirectory 'jump-list-icons.png'), [System.Drawing.Imaging.ImageFormat]::Png)
        } finally {
            $graphics.Dispose(); $font.Dispose(); $atlas.Dispose()
            foreach ($brush in ($brushes + $textBrushes)) { $brush.Dispose() }
        }
    }
} finally {
    foreach ($bitmap in $bitmaps) { $bitmap.Dispose() }
}
