<#
.SYNOPSIS
  Voquill bootstrap installer for Windows.
.DESCRIPTION
  Detects architecture, downloads the matching setup.exe (default user install,
  no admin needed) or MSI (system-wide IT install) from the latest GitHub release,
  and runs the installer. Supports override URLs for testing.
.PARAMETER System
  Install system-wide via MSI (requires administrator privileges).
.PARAMETER Version
  Install a specific release tag (e.g. "v1.5.0").
.PARAMETER InsecureSkipVerify
  Skip checksum verification (not recommended).
.PARAMETER Help
  Show this help message.
.EXAMPLE
  irm https://voquill.org/install.ps1 | iex
.EXAMPLE
  irm https://voquill.org/install.ps1 | iex -args "-System"
.EXAMPLE
  irm https://voquill.org/install.ps1 | iex -args "-Version v1.5.0"
#>

param(
  [string]$Version = "",
  [switch]$System,
  [switch]$InsecureSkipVerify,
  [switch]$Help
)

if ($Help) {
  Get-Help $MyInvocation.MyCommand.Path
  exit 0
}

$Repo = "jackbrumley/voquill"
$AppName = "voquill"
$TempDir = Join-Path $env:TEMP "voquill-install"

function Log($Message) {
  Write-Host "[voquill] $Message"
}

function Fail($Message) {
  Write-Host "[voquill] ERROR: $Message" -ForegroundColor Red
  exit 1
}

function Get-Architecture {
  $arch = $env:PROCESSOR_ARCHITECTURE
  if ($arch -eq "AMD64") { return "x64" }
  if ($arch -eq "ARM64") { return "arm64" }
  Fail "Unsupported architecture: $arch"
}

function Get-LatestReleaseTag {
  $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
  try {
    $response = Invoke-RestMethod -Uri $apiUrl -Headers @{ "Accept" = "application/json" } -ErrorAction Stop
    return $response.tag_name
  } catch {
    Fail "Could not determine latest release tag from GitHub API: $_"
  }
}

function Get-AssetDownloadUrl($Tag, $AssetName) {
  $apiUrl = "https://api.github.com/repos/$Repo/releases/tags/$Tag"
  try {
    $response = Invoke-RestMethod -Uri $apiUrl -Headers @{ "Accept" = "application/json" } -ErrorAction Stop
    foreach ($asset in $response.assets) {
      if ($asset.name -eq $AssetName) {
        return $asset.browser_download_url
      }
    }
  } catch {
    Fail "Could not find asset '$AssetName' in release $Tag': $_"
  }
  Fail "Asset '$AssetName' not found in release $Tag"
}

function Get-Checksum($Path) {
  $hash = Get-FileHash -Path $Path -Algorithm SHA256
  return $hash.Hash.ToLower()
}

# Determine install type: setup.exe (default user-level, no admin needed) or MSI (system-wide)
if ($System) {
  $InstallType = "msi"
} else {
  $InstallType = "exe"
}

# Resolve version
if ([string]::IsNullOrEmpty($Version)) {
  $ReleaseTag = Get-LatestReleaseTag
} else {
  $ReleaseTag = $Version
}

$Arch = Get-Architecture
$Os = "windows"
$VersionTag = $ReleaseTag.TrimStart('v')

# Resolve asset name
if ($InstallType -eq "msi") {
  $AssetName = "${AppName}-${VersionTag}-${Os}-${Arch}.msi"
} else {
  $AssetName = "${AppName}-${VersionTag}-${Os}-${Arch}-setup.exe"
}

Log "Preparing Voquill install"
Log "Release: $ReleaseTag"
Log "Architecture: $Arch"
Log "Asset: $AssetName"

$DownloadUrl = Get-AssetDownloadUrl -Tag $ReleaseTag -AssetName $AssetName
$ChecksumUrl = "${DownloadUrl}.sha256"

if (Test-Path $TempDir) { Remove-Item -Recurse -Force $TempDir }
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

$InstallerPath = Join-Path $TempDir $AssetName
$ChecksumPath = Join-Path $TempDir "${AssetName}.sha256"

Log "Downloading release artifact"
try {
  Invoke-WebRequest -Uri $DownloadUrl -OutFile $InstallerPath -ErrorAction Stop
} catch {
  Fail "Download failed: $_"
}

if (-not $InsecureSkipVerify) {
  Log "Downloading checksum"
  try {
    Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumPath -ErrorAction Stop
  } catch {
    Log "WARNING: checksum download failed, skipping verification"
  }

  if (Test-Path $ChecksumPath) {
    $ExpectedHash = (Get-Content $ChecksumPath).Split(' ')[0].Trim()
    if ([string]::IsNullOrEmpty($ExpectedHash)) {
      Log "WARNING: checksum file was empty, skipping verification"
    } else {
      $ActualHash = Get-Checksum $InstallerPath
      if ($ExpectedHash -ne $ActualHash) {
        Fail "Checksum mismatch`n  expected: $ExpectedHash`n  actual:   $ActualHash"
      }
      Log "Checksum verified"
    }
  }
} else {
  Log "WARNING: checksum verification disabled"
}

Log "Running installer"
try {
  if ($InstallType -eq "msi") {
    $process = Start-Process -FilePath "msiexec.exe" -ArgumentList "/i `"$InstallerPath`" /qb /norestart" -Wait -PassThru
    if ($process.ExitCode -ne 0 -and $process.ExitCode -ne 3010) {
      Fail "MSI installer failed with exit code $($process.ExitCode)"
    }
  } else {
    $process = Start-Process -FilePath $InstallerPath -ArgumentList "/S" -Wait -PassThru
    if ($process.ExitCode -ne 0) {
      Fail "Installer failed with exit code $($process.ExitCode)"
    }
  }
} catch {
  Fail "Failed to start installer: $_"
}

Remove-Item -Recurse -Force $TempDir

Log "Installation complete"
Log "Voquill was installed. Launch it from your Start Menu."

if ($InstallType -eq "msi" -and $process.ExitCode -eq 3010) {
  Log "NOTE: A system restart is recommended to complete the installation."
}