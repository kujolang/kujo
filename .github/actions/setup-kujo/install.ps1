param(
  [Parameter(Mandatory = $true)][string]$Version,
  [Parameter(Mandatory = $true)][string]$InstallDirectory
)
$ErrorActionPreference = "Stop"
if ($Version -notmatch '^v\d+\.\d+\.\d+$') { throw "setup-kujo: version must be an exact vMAJOR.MINOR.PATCH release tag" }
if ($env:RUNNER_ARCH -ne "X64") { throw "setup-kujo: unsupported Windows architecture $env:RUNNER_ARCH" }

$Asset = "kujo-$Version-windows-x64.zip"
$ReleaseUrl = "https://github.com/kujolang/kujo/releases/tag/$Version"
$BaseUrl = "https://github.com/kujolang/kujo/releases/download/$Version"
$BinaryPath = Join-Path $InstallDirectory "kujo.exe"
$MetadataPath = Join-Path $InstallDirectory "setup-kujo.json"
New-Item -ItemType Directory -Force $InstallDirectory | Out-Null

if (-not (Test-Path $BinaryPath) -or -not (Test-Path $MetadataPath)) {
  $WorkDirectory = Join-Path $env:RUNNER_TEMP ("setup-kujo-" + [guid]::NewGuid().ToString("N"))
  New-Item -ItemType Directory -Force $WorkDirectory | Out-Null
  try {
    $ArchivePath = Join-Path $WorkDirectory $Asset
    $ChecksumPath = "$ArchivePath.sha256"
    Invoke-WebRequest "$BaseUrl/$Asset" -OutFile $ArchivePath
    Invoke-WebRequest "$BaseUrl/$Asset.sha256" -OutFile $ChecksumPath
    $Expected = ((Get-Content $ChecksumPath -First 1) -split '\s+')[0].ToLowerInvariant()
    $Actual = (Get-FileHash -Algorithm SHA256 $ArchivePath).Hash.ToLowerInvariant()
    if ($Expected -notmatch '^[0-9a-f]{64}$') { throw "setup-kujo: release checksum is malformed" }
    if ($Expected -ne $Actual) { throw "setup-kujo: checksum mismatch for $Asset" }
    Expand-Archive $ArchivePath -DestinationPath $InstallDirectory -Force
    @{ version = $Version; checksum = $Expected; asset = $Asset; provenance_url = $ReleaseUrl } |
      ConvertTo-Json -Compress | Set-Content $MetadataPath
  } finally {
    Remove-Item -Recurse -Force $WorkDirectory -ErrorAction SilentlyContinue
  }
}

$Metadata = Get-Content $MetadataPath | ConvertFrom-Json
if ($Metadata.version -ne $Version) { throw "setup-kujo: cached runtime version mismatch" }
$Reported = & $BinaryPath --version
if ($Reported -notmatch [regex]::Escape($Version.Substring(1))) { throw "setup-kujo: installed binary version mismatch" }
$InstallDirectory | Out-File -FilePath $env:GITHUB_PATH -Append
"kujo-path=$BinaryPath" | Out-File -FilePath $env:GITHUB_OUTPUT -Append
"checksum=$($Metadata.checksum)" | Out-File -FilePath $env:GITHUB_OUTPUT -Append
"asset=$($Metadata.asset)" | Out-File -FilePath $env:GITHUB_OUTPUT -Append
"provenance-url=$($Metadata.provenance_url)" | Out-File -FilePath $env:GITHUB_OUTPUT -Append
