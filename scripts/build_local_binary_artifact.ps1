param(
    [switch]$Install,
    [string]$InstallDir = "$HOME\.local\bin",
    [string]$Version
)

$ErrorActionPreference = "Stop"

function Write-Usage {
    @"
Usage: pwsh -File scripts/build_local_binary_artifact.ps1 [-Install] [-InstallDir <dir>] [-Version <version>]

Builds a local release binary, packages it as a platform archive, writes a SHA-256
checksum, and optionally installs the binary into a user directory.

Options:
  -Install            Copy the built binary into the install directory.
  -InstallDir <dir>   Install destination for -Install. Default: $HOME\.local\bin
  -Version <version>  Override the artifact version prefix. Default: Cargo package version.
"@
}

if ($args -contains "-h" -or $args -contains "--help") {
    Write-Usage
    exit 0
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $repoRoot

function Get-PathEntries {
    if ([string]::IsNullOrWhiteSpace($env:PATH)) {
        return @()
    }

    return $env:PATH.Split([IO.Path]::PathSeparator, [System.StringSplitOptions]::RemoveEmptyEntries)
}

function Test-PathContainsDir {
    param([string]$TargetDir)

    $normalizedTarget = [IO.Path]::GetFullPath($TargetDir).TrimEnd('\')
    foreach ($entry in Get-PathEntries) {
        try {
            $normalizedEntry = [IO.Path]::GetFullPath($entry).TrimEnd('\')
            if ($normalizedEntry -ieq $normalizedTarget) {
                return $true
            }
        } catch {
            continue
        }
    }

    return $false
}

$osName = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
$archName = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString().ToLowerInvariant()

if (-not $IsWindows) {
    throw "[local-artifact] ERROR: this PowerShell installer is intended for Windows. Use scripts/build_local_binary_artifact.sh on macOS or Linux."
}

switch ($archName) {
    "x64" {
        $platform = "windows-x64"
        $binaryName = "kujo.exe"
        $archiveExt = "zip"
    }
    default {
        throw "[local-artifact] ERROR: unsupported Windows architecture: $archName"
    }
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    $cargoToml = Get-Content (Join-Path $repoRoot "Cargo.toml")
    foreach ($line in $cargoToml) {
        if ($line -match '^version = "(.*)"$') {
            $Version = $Matches[1]
            break
        }
    }
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    throw "[local-artifact] ERROR: unable to determine package version from Cargo.toml"
}

$buildDate = Get-Date -Format "yyyyMMdd"
$commitSha = (git rev-parse --short HEAD).Trim()
$artifactRoot = Join-Path $repoRoot "target\local-artifacts"
$distDir = Join-Path $artifactRoot "dist"
$archiveName = "kujo-v$Version-$buildDate-$commitSha-$platform.$archiveExt"
$archivePath = Join-Path $artifactRoot $archiveName
$checksumPath = "$archivePath.sha256"
$binaryPath = Join-Path $repoRoot "target\release\$binaryName"

New-Item -ItemType Directory -Force $artifactRoot | Out-Null
New-Item -ItemType Directory -Force $distDir | Out-Null

foreach ($path in @((Join-Path $distDir $binaryName), $archivePath, $checksumPath)) {
    if (Test-Path $path) {
        Remove-Item $path -Force
    }
}

Write-Host "[local-artifact] building release binary"
cargo build --release --locked

if (-not (Test-Path $binaryPath)) {
    throw "[local-artifact] ERROR: expected binary not found at $binaryPath"
}

Write-Host "[local-artifact] smoke testing release binary"
& $binaryPath --version
& $binaryPath run "examples/hello.kujo"

Copy-Item $binaryPath (Join-Path $distDir $binaryName) -Force

Write-Host "[local-artifact] packaging $archiveName"
if (Test-Path $archivePath) {
    Remove-Item $archivePath -Force
}
Compress-Archive -Path (Join-Path $distDir $binaryName) -DestinationPath $archivePath -Force

Write-Host "[local-artifact] writing checksum"
$hash = (Get-FileHash -Algorithm SHA256 $archivePath).Hash.ToLowerInvariant()
"$hash  $archiveName" | Set-Content -NoNewline $checksumPath

if ($Install) {
    New-Item -ItemType Directory -Force $InstallDir | Out-Null
    Copy-Item $binaryPath (Join-Path $InstallDir "kujo.exe") -Force
    Write-Host "[local-artifact] installed $(Join-Path $InstallDir 'kujo.exe')"
    & (Join-Path $InstallDir "kujo.exe") --version

    if (Test-PathContainsDir -TargetDir $InstallDir) {
        Write-Host "[local-artifact] PATH already includes $InstallDir; you can run: kujo --version"
    } else {
        Write-Host "[local-artifact] WARNING: $InstallDir is not on PATH in this shell."
        Write-Host "[local-artifact] Add it to your user PATH, then open a new shell."
    }
}

Write-Host "[local-artifact] artifact: $archivePath"
Write-Host "[local-artifact] checksum: $checksumPath"
