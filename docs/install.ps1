<#
.SYNOPSIS
  Voquill bootstrap installer for Windows.
.DESCRIPTION
  Detects architecture, downloads the matching setup.exe (default user install,
  no admin needed) or MSI (system-wide IT install) from the latest GitHub release,
  stops running instances, cleanly replaces previous installations, and runs the installer.
.PARAMETER System
  Install system-wide via MSI (requires administrator privileges).
.PARAMETER Clean
  Remove cached data (models, python-runner, debug) before installing. Keeps config.json and history.db.
.PARAMETER Version
  Install a specific release tag (e.g. "v1.5.0").
.PARAMETER InsecureSkipVerify
  Skip checksum verification (not recommended).
.PARAMETER Relaunch
  Launch Voquill after successful installation.
.PARAMETER Help
  Show this help message.
.EXAMPLE
  irm https://voquill.org/install.ps1 | iex
.EXAMPLE
  irm https://voquill.org/install.ps1 | iex -args "-System"
.EXAMPLE
  irm https://voquill.org/install.ps1 | iex -args "-Version v1.5.0"
.EXAMPLE
  irm https://voquill.org/install.ps1 | iex -args "-Relaunch"
#>

param(
  [string]$Version = "",
  [switch]$System,
  [switch]$Clean,
  [switch]$InsecureSkipVerify,
  [switch]$Relaunch,
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

function Stop-RunningProcess {
  $runningProcesses = Get-Process -Name "voquill" -ErrorAction SilentlyContinue
  if ($runningProcesses) {
    Log "Stopping running Voquill process"
    Stop-Process -Name "voquill" -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
  }
}

function Uninstall-ExistingVoquill {
  Log "Checking for previous Voquill installation"
  $RegistryPaths = @(
    "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
    "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*"
  )

  foreach ($path in $RegistryPaths) {
    $entries = Get-ItemProperty -Path $path -ErrorAction SilentlyContinue | Where-Object {
      $_.DisplayName -and ($_.DisplayName -like "*Voquill*" -or $_.Publisher -like "*Voquill*" -or $_.Publisher -like "*Jack Brumley*")
    }

    foreach ($entry in $entries) {
      Log "Uninstalling previous version: $($entry.DisplayName)"

      $uninstallString = $entry.UninstallString
      $quietUninstallString = $entry.QuietUninstallString

      if ($entry.PSChildName -match "^\{[0-9A-Fa-f\-]+\}$") {
        # MSI Product Code
        $guid = $entry.PSChildName
        Log "Running MSI uninstaller for product code $guid"
        try {
          $process = Start-Process -FilePath "msiexec.exe" -ArgumentList "/x `"$guid`" /qb /norestart" -Wait -PassThru
          if ($process.ExitCode -ne 0 -and $process.ExitCode -ne 3010) {
            Log "WARNING: MSI uninstaller returned exit code $($process.ExitCode)"
          }
        } catch {
          Log "WARNING: Failed to run msiexec for $guid`: $_"
        }
      } elseif ($quietUninstallString) {
        Log "Running quiet uninstaller: $quietUninstallString"
        try {
          $process = Start-Process -FilePath "cmd.exe" -ArgumentList "/c `"$quietUninstallString`"" -Wait -PassThru
        } catch {
          Log "WARNING: Failed to run quiet uninstaller: $_"
        }
      } elseif ($uninstallString) {
        if ($uninstallString -match "msiexec" -and $uninstallString -match "(\{[0-9A-Fa-f\-]+\})") {
          $guid = $Matches[1]
          Log "Running MSI uninstaller for product code $guid"
          try {
            $process = Start-Process -FilePath "msiexec.exe" -ArgumentList "/x `"$guid`" /qb /norestart" -Wait -PassThru
          } catch {
            Log "WARNING: Failed to run msiexec: $_"
          }
        } else {
          Log "Running uninstaller command: $uninstallString"
          try {
            $exe = $uninstallString.Trim('"')
            if (Test-Path $exe) {
              $process = Start-Process -FilePath $exe -ArgumentList "/S" -Wait -PassThru
            } else {
              $process = Start-Process -FilePath "cmd.exe" -ArgumentList "/c `"$uninstallString /S`"" -Wait -PassThru
            }
          } catch {
            Log "WARNING: Failed to run uninstaller string: $_"
          }
        }
      }
    }
  }

  # Clean up Start Menu and Desktop shortcuts
  $ShortcutPaths = @(
    (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Voquill.lnk"),
    (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\voquill.lnk"),
    (Join-Path $env:USERPROFILE "Desktop\Voquill.lnk"),
    (Join-Path $env:USERPROFILE "Desktop\voquill.lnk"),
    (Join-Path $env:ProgramData "Microsoft\Windows\Start Menu\Programs\Voquill.lnk"),
    (Join-Path $env:Public "Desktop\Voquill.lnk")
  )

  foreach ($shortcut in $ShortcutPaths) {
    if (Test-Path $shortcut) {
      Remove-Item -Path $shortcut -Force -ErrorAction SilentlyContinue
    }
  }
}

function Clean-UserData {
  $UserDataDir = Join-Path $env:USERPROFILE ".config\voquill-app"
  if (-not (Test-Path $UserDataDir)) {
    return
  }
  Log "Cleaning cached user data (keeping config.json and history.db)"
  foreach ($dir in @("models", "python-runner", "debug")) {
    $target = Join-Path $UserDataDir $dir
    if (Test-Path $target) {
      Log "  Removing ${dir}/"
      Remove-Item -Path $target -Recurse -Force -ErrorAction SilentlyContinue
    }
  }
  Log "User data cleaned"
}

# Determine install type: setup.exe (default user-level, no admin needed) or MSI (system-wide)
if ($System -or $env:VOQUILL_SYSTEM -eq "1" -or $env:VOQUILL_SYSTEM -eq "true") {
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

# Stop running process and remove previous installation before deploying new version
Stop-RunningProcess
Uninstall-ExistingVoquill

if ($Clean -or $env:VOQUILL_CLEAN -eq "1" -or $env:VOQUILL_CLEAN -eq "true") {
  Log "Clean mode enabled"
  Clean-UserData
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

if ($Relaunch) {
  Log "Relaunching Voquill..."
  $possiblePaths = @(
    (Join-Path $env:LOCALAPPDATA "Programs\Voquill\Voquill.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\Voquill\voquill.exe"),
    (Join-Path $env:ProgramFiles "Voquill\Voquill.exe"),
    (Join-Path $env:ProgramFiles "Voquill\voquill.exe"),
    (Join-Path ${env:ProgramFiles(x86)} "Voquill\Voquill.exe")
  )
  $started = $false
  foreach ($p in $possiblePaths) {
    if ($p -and (Test-Path $p)) {
      Start-Process -FilePath $p
      $started = $true
      break
    }
  }
  if (-not $started) {
    $shortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Voquill.lnk"
    if (Test-Path $shortcut) {
      Start-Process -FilePath $shortcut
    }
  }
}