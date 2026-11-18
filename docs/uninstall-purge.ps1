# Older Windows 10 PCs offer only TLS 1.0 by default, which GitHub refuses.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$env:VOQUILL_PURGE_DATA = "1"
$env:VOQUILL_YES = "1"
irm https://voquill.org/uninstall.ps1 | iex
