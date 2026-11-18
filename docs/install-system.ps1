# Older Windows 10 PCs offer only TLS 1.0 by default, which GitHub refuses.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$env:VOQUILL_SYSTEM = "1"
irm https://voquill.org/install.ps1 | iex
