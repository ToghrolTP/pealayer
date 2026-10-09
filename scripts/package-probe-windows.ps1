function Invoke-PackageProbe([string]$Executable, [string]$Arguments, [string]$WorkingDirectory) {
    # Retain the handle: Start-Process can lose a short-lived GUI exe's exit code.
    $probe = [System.Diagnostics.Process]::new()
    try {
        $probe.StartInfo.FileName = $Executable
        $probe.StartInfo.Arguments = $Arguments
        $probe.StartInfo.WorkingDirectory = $WorkingDirectory
        $probe.StartInfo.UseShellExecute = $false
        $probe.StartInfo.CreateNoWindow = $true
        $probe.StartInfo.RedirectStandardOutput = $true
        [void]$probe.Start()
        $probeOutput = $probe.StandardOutput.ReadToEndAsync()
        if (-not $probe.WaitForExit(30000)) { throw "Package probe timed out: $Arguments" }
        if ($probe.ExitCode -ne 0) { throw "Package probe failed ($($probe.ExitCode)): $Arguments" }
        return $probeOutput.GetAwaiter().GetResult()
    } finally { $probe.Dispose() }
}
