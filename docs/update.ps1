# Voquill's in-app updater for Windows.
#
# Update Now (from 1.11.5 on) downloads the current copy of this script and
# runs it, so this file owns the whole update. A fix made here reaches every
# installed version on its next update, with no new release. The app only
# starts PowerShell and waits for the started line below.
#
# Contract with the app; keep it stable, every shipped version relies on it:
#   - Parameters: -BaseUrl (where this script was downloaded from, without a
#     trailing slash) and -Relaunch (the installed voquill.exe).
#   - The first output line is exactly "[voquill-update] started". Until the
#     app sees it in debug\update.log, it stays open; if the line never comes,
#     it tells the user and offers the download page instead of closing.
#   - Output goes to debug\update.log (the app redirects it there).
#   - The last line is "[voquill-update] finished: updated" or
#     "[voquill-update] finished: failed". On failure the installed version
#     is relaunched with --update-failed, which shows the user a dialog.
param(
  [Parameter(Mandatory = $true)][string]$BaseUrl,
  [string]$Relaunch = ""
)

Write-Output "[voquill-update] started"
Write-Output "[voquill-update] $(Get-Date -Format o) from $BaseUrl"

# install.ps1 runs from a file rather than through iex, so its `exit 1` on
# failure ends that script, not this one, and sets $LASTEXITCODE.
$installer = Join-Path $env:TEMP 'voquill-update-install.ps1'
try {
  Invoke-RestMethod -Uri "$BaseUrl/install.ps1" -OutFile $installer -ErrorAction Stop
  $global:LASTEXITCODE = 0
  & $installer
  $succeeded = $global:LASTEXITCODE -eq 0
} catch {
  Write-Output "[voquill-update] ERROR: $_"
  $succeeded = $false
}

if ($succeeded) {
  Write-Output "[voquill-update] finished: updated"
  exit 0
}

Write-Output "[voquill-update] finished: failed"
if ($Relaunch -and (Test-Path -LiteralPath $Relaunch)) {
  # Voquill exits once it sees the started line. Until it has, a relaunch
  # would hand off to the exiting instance (single instance) and vanish.
  Get-Process -Name voquill -ErrorAction SilentlyContinue | Wait-Process -Timeout 15 -ErrorAction SilentlyContinue
  Write-Output "[voquill-update] Relaunching the installed version"
  Start-Process -FilePath $Relaunch -ArgumentList '--update-failed'
} else {
  Write-Output "[voquill-update] No installed version found to relaunch"
}
exit 1
