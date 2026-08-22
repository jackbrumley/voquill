<#
.SYNOPSIS
  Voquill uninstaller script for Windows.
.DESCRIPTION
  Stops running Voquill processes, runs the Windows uninstaller (MSI/NSIS),
  cleans up shortcuts, and optionally purges user data.
.PARAMETER PurgeData
  Also remove ~/.config/voquill-app (models, config, history, logs).
.PARAMETER Yes
  Run non-interactively (skip confirmation prompt).
.PARAMETER Help
  Show this help message.
.EXAMPLE
  irm https://voquill.org/uninstall.ps1 | iex
.EXAMPLE
  irm https://voquill.org/uninstall.ps1 | iex -args "-PurgeData -Yes"
#>

param(
  [switch]$PurgeData,
  [switch]$Yes,
  [switch]$Help
)

if ($Help) {
  Get-Help $MyInvocation.MyCommand.Path
  exit 0
}

function Log($Message) {
  Write-Host "[voquill-uninstall] $Message"
}

function Fail($Message) {
  Write-Host "[voquill-uninstall] ERROR: $Message" -ForegroundColor Red
  exit 1
}

# Determine interactive mode
$NonInteractive = $Yes
if (-not $NonInteractive) {
  # If executed via pipeline (e.g. iex), stdin may not be available for interactive prompt
  if (-not [Console]::IsInputRedirected -and $Host.Name -notmatch "Server") {
    # Interactive session
  } else {
    $NonInteractive = $true
  }
}

Log "Preparing Voquill uninstall"

if (-not $NonInteractive) {
  $answer = Read-Host "Proceed with uninstall? [y/N]"
  if ($answer -ne "y" -and $answer -ne "Y") {
    Fail "uninstall cancelled"
  }
} else {
  Log "Non-interactive mode detected"
}

# Stop running process
$runningProcesses = Get-Process -Name "voquill" -ErrorAction SilentlyContinue
if ($runningProcesses) {
  Log "Closing running Voquill process"
  Stop-Process -Name "voquill" -Force -ErrorAction SilentlyContinue
  Start-Sleep -Seconds 1
}

# Find installed Voquill package via Windows Registry
$RegistryPaths = @(
  "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*",
  "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*",
  "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*"
)

$uninstalledAny = $false

foreach ($path in $RegistryPaths) {
  $entries = Get-ItemProperty -Path $path -ErrorAction SilentlyContinue | Where-Object {
    $_.DisplayName -and ($_.DisplayName -like "*Voquill*" -or $_.Publisher -like "*Voquill*" -or $_.Publisher -like "*Jack Brumley*")
  }

  foreach ($entry in $entries) {
    Log "Found installed package: $($entry.DisplayName)"
    $uninstalledAny = $true

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
          # Strip quotes if present for executable path
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

if (-not $uninstalledAny) {
  Log "No registered Voquill installation found in Windows Registry."
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
    Log "Removing shortcut: $shortcut"
    Remove-Item -Path $shortcut -Force -ErrorAction SilentlyContinue
  }
}

# User data handling
$UserDataDir = Join-Path $env:USERPROFILE ".config\voquill-app"
$LegacyDataDir = Join-Path $env:APPDATA "foss-voquill"

if ($PurgeData) {
  if (Test-Path $UserDataDir) {
    Log "Removing user data: $UserDataDir"
    Remove-Item -Path $UserDataDir -Recurse -Force -ErrorAction SilentlyContinue
  }
  if (Test-Path $LegacyDataDir) {
    Log "Removing legacy user data: $LegacyDataDir"
    Remove-Item -Path $LegacyDataDir -Recurse -Force -ErrorAction SilentlyContinue
  }
} else {
  if (Test-Path $UserDataDir) {
    Log "User data left intact at $UserDataDir"
    Log "To remove it manually: Remove-Item -Recurse -Force '$UserDataDir'"
  }
}

Log "Uninstall complete"
if ($PurgeData) {
  Log "Voquill and all user data were removed."
} else {
  Log "Voquill was removed. User data (~/.config/voquill-app) was left intact."
  Log "Re-run with -PurgeData to remove user data as well."
}
