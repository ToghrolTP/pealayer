Set-StrictMode -Version Latest

function Get-PealayerWindowsHostPaths {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$RepositoryRoot
    )

    $programRoot = Join-Path $env:LOCALAPPDATA 'Programs\Pealayer'
    [pscustomobject]@{
        ProgramRoot = $programRoot
        ProfileDirectory = Join-Path $programRoot 'config'
        ProfilePath = Join-Path $programRoot 'config\windows-build-host.json'
        LinkDirectory = Join-Path $programRoot 'build-dependencies\libmpv'
        CargoConfigPath = Join-Path $RepositoryRoot '.cargo\config.toml'
    }
}

function Get-PealayerFileIdentity {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $item = Get-Item -LiteralPath $Path -ErrorAction Stop
    [ordered]@{
        path = $item.FullName
        file_name = $item.Name
        bytes = $item.Length
        sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        file_version = if ($item.VersionInfo.FileVersion) { $item.VersionInfo.FileVersion } else { $null }
        product_version = if ($item.VersionInfo.ProductVersion) { $item.VersionInfo.ProductVersion } else { $null }
    }
}

function Resolve-PealayerLibmpv {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory = $true)]
        [string]$RustHost,
        [string]$ExplicitDirectory
    )

    $paths = Get-PealayerWindowsHostPaths -RepositoryRoot $RepositoryRoot
    $candidates = [System.Collections.Generic.List[object]]::new()
    $seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    function Add-Candidate([string]$Directory, [string]$Source) {
        if ([string]::IsNullOrWhiteSpace($Directory)) { return }
        $expanded = [Environment]::ExpandEnvironmentVariables($Directory.Trim())
        try { $full = [System.IO.Path]::GetFullPath($expanded) } catch { return }
        if ($seen.Add($full)) {
            $candidates.Add([pscustomobject]@{ Directory = $full; Source = $Source })
        }
    }

    Add-Candidate $ExplicitDirectory 'parameter'
    Add-Candidate $env:LIBMPV_DIR 'process environment'

    if (Test-Path -LiteralPath $paths.ProfilePath -PathType Leaf) {
        try {
            $profile = Get-Content -Raw -LiteralPath $paths.ProfilePath | ConvertFrom-Json
            if (-not $profile.host -or [string]$profile.host -ieq $env:COMPUTERNAME) {
                Add-Candidate ([string]$profile.libmpv_source_directory) 'host profile'
            }
        } catch {
            Write-Warning "Ignoring unreadable Pealayer Windows host profile at $($paths.ProfilePath): $($_.Exception.Message)"
        }
    }

    Add-Candidate ([Environment]::GetEnvironmentVariable('LIBMPV_DIR', 'User')) 'user environment'
    Add-Candidate ([Environment]::GetEnvironmentVariable('LIBMPV_DIR', 'Machine')) 'machine environment'
    Add-Candidate (Join-Path $env:ProgramFiles 'MPV') 'Program Files default'

    $attempted = [System.Collections.Generic.List[string]]::new()
    foreach ($candidate in $candidates) {
        $sourceDirectory = $candidate.Directory
        if (-not (Test-Path -LiteralPath $sourceDirectory -PathType Container)) {
            $attempted.Add("$sourceDirectory (missing directory)")
            continue
        }

        $runtime = @('libmpv-2.dll', 'mpv-2.dll') |
            ForEach-Object { Join-Path $sourceDirectory $_ } |
            Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
            Select-Object -First 1
        if (-not $runtime) {
            $attempted.Add("$sourceDirectory (missing libmpv-2.dll/mpv-2.dll)")
            continue
        }

        $linkDirectory = $sourceDirectory
        $importLibrary = $null
        $importSource = $null
        if ($RustHost -like '*-msvc') {
            $nativeImport = Join-Path $sourceDirectory 'mpv.lib'
            $gnuImport = Join-Path $sourceDirectory 'libmpv.dll.a'
            if (Test-Path -LiteralPath $nativeImport -PathType Leaf) {
                $importLibrary = $nativeImport
                $importSource = $nativeImport
            } elseif (Test-Path -LiteralPath $gnuImport -PathType Leaf) {
                New-Item -ItemType Directory -Force -Path $paths.LinkDirectory | Out-Null
                $stagedImport = Join-Path $paths.LinkDirectory 'mpv.lib'
                $copyRequired = -not (Test-Path -LiteralPath $stagedImport -PathType Leaf)
                if (-not $copyRequired) {
                    $copyRequired = (Get-FileHash -LiteralPath $gnuImport -Algorithm SHA256).Hash -ne
                        (Get-FileHash -LiteralPath $stagedImport -Algorithm SHA256).Hash
                }
                if ($copyRequired) {
                    Copy-Item -LiteralPath $gnuImport -Destination $stagedImport -Force
                }
                $linkDirectory = $paths.LinkDirectory
                $importLibrary = $stagedImport
                $importSource = $gnuImport
            }
        } else {
            $importLibrary = @('libmpv.dll.a', 'libmpv.a') |
                ForEach-Object { Join-Path $sourceDirectory $_ } |
                Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
                Select-Object -First 1
            $importSource = $importLibrary
        }

        if (-not $importLibrary) {
            $expected = if ($RustHost -like '*-msvc') { 'mpv.lib or libmpv.dll.a' } else { 'libmpv.dll.a or libmpv.a' }
            $attempted.Add("$sourceDirectory (missing $expected)")
            continue
        }

        return [pscustomobject]@{
            Host = $env:COMPUTERNAME
            RustHost = $RustHost
            ResolutionSource = $candidate.Source
            SourceDirectory = $sourceDirectory
            LinkDirectory = $linkDirectory
            ImportLibrary = $importLibrary
            ImportSource = $importSource
            RuntimeLibrary = $runtime
            Paths = $paths
        }
    }

    $details = if ($attempted.Count) { "`n - " + ($attempted -join "`n - ") } else { ' no candidates were available' }
    throw "No complete libmpv development package was found for $RustHost. Checked:$details`nRun scripts\configure-windows-host.ps1 -LibmpvDirectory <directory> once on this host."
}

function Set-PealayerLibmpvBuildEnvironment {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [object]$Resolution
    )

    $env:LIBMPV_DIR = $Resolution.SourceDirectory
    $pathParts = @($env:Path -split ';')
    foreach ($directory in @($Resolution.LinkDirectory, $Resolution.SourceDirectory)) {
        if ($pathParts -notcontains $directory) {
            $env:Path = $directory + ';' + $env:Path
            $pathParts = @($directory) + $pathParts
        }
    }
    $separator = [char]0x1f
    $linkFlag = "-Lnative=$($Resolution.LinkDirectory)"
    $existingFlags = @()
    if ($env:CARGO_ENCODED_RUSTFLAGS) {
        $existingFlags = @($env:CARGO_ENCODED_RUSTFLAGS -split [string]$separator)
    }
    if ($existingFlags -notcontains $linkFlag) {
        $env:CARGO_ENCODED_RUSTFLAGS = (@($existingFlags) + $linkFlag | Where-Object { $_ }) -join $separator
    }
}

function Save-PealayerWindowsHostProfile {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory = $true)]
        [object]$Resolution,
        [switch]$PersistUserEnvironment
    )

    $paths = $Resolution.Paths
    New-Item -ItemType Directory -Force -Path $paths.ProfileDirectory | Out-Null
    $profile = [ordered]@{
        format = 'pealayer-windows-build-host'
        host = $env:COMPUTERNAME
        configured_at_utc = [DateTime]::UtcNow.ToString('o')
        rust_host = $Resolution.RustHost
        libmpv_source_directory = $Resolution.SourceDirectory
        libmpv_link_directory = $Resolution.LinkDirectory
        resolution_source = $Resolution.ResolutionSource
        import_library = Get-PealayerFileIdentity -Path $Resolution.ImportLibrary
        import_source = Get-PealayerFileIdentity -Path $Resolution.ImportSource
        runtime = Get-PealayerFileIdentity -Path $Resolution.RuntimeLibrary
    }
    $profile | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $paths.ProfilePath -Encoding utf8

    $cargoDirectory = Split-Path -Parent $paths.CargoConfigPath
    New-Item -ItemType Directory -Force -Path $cargoDirectory | Out-Null
    if (Test-Path -LiteralPath $paths.CargoConfigPath -PathType Leaf) {
        $firstLine = Get-Content -LiteralPath $paths.CargoConfigPath -TotalCount 1
        if ($firstLine -ne '# Generated by scripts/configure-windows-host.ps1; do not commit.') {
            throw "Refusing to overwrite user-managed Cargo configuration: $($paths.CargoConfigPath)"
        }
    }
    $linkFlagToml = ("-Lnative=$($Resolution.LinkDirectory)" | ConvertTo-Json -Compress)
    @(
        '# Generated by scripts/configure-windows-host.ps1; do not commit.'
        "# Host: $env:COMPUTERNAME"
        "# libmpv runtime: $($profile.runtime.file_version) ($($profile.runtime.sha256))"
        "[target.$($Resolution.RustHost)]"
        "rustflags = [$linkFlagToml]"
        ''
    ) | Set-Content -LiteralPath $paths.CargoConfigPath -Encoding utf8

    if ($PersistUserEnvironment) {
        [Environment]::SetEnvironmentVariable('LIBMPV_DIR', $Resolution.SourceDirectory, 'User')
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $userPathParts = if ($userPath) { @($userPath -split ';' | Where-Object { $_ }) } else { @() }
        if ($userPathParts -notcontains $Resolution.SourceDirectory) {
            $updatedUserPath = (@($userPathParts) + $Resolution.SourceDirectory) -join ';'
            [Environment]::SetEnvironmentVariable('Path', $updatedUserPath, 'User')
        }
    }

    return [pscustomobject]@{
        Profile = $profile
        ProfilePath = $paths.ProfilePath
        CargoConfigPath = $paths.CargoConfigPath
        UserEnvironmentPersisted = [bool]$PersistUserEnvironment
    }
}
