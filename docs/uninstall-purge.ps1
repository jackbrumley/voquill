$env:VOQUILL_PURGE_DATA = "1"
$env:VOQUILL_YES = "1"
irm https://voquill.org/uninstall.ps1 | iex
