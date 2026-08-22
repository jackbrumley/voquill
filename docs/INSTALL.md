---
layout: default
title: Voquill Install Guide
description: One-command install of Voquill on Linux and Windows. System package, AppImage, or PowerShell.
canonical: https://voquill.org/INSTALL.html
body_class: centered-page
container_class: home-container
---

# Voquill Install Guide

Voquill supports a one-command install flow from the website, matching the uninstall UX.

## User Install (Recommended — No Admin/Sudo Required)

Installs Voquill in the current user context. Zero administrator privileges or `sudo` required.

**Windows (PowerShell):**
```powershell
irm https://voquill.org/install.ps1 | iex
```

**Linux (AppImage):**
```bash
curl -sf https://voquill.org/install.sh | bash
```

## System Install (IT / Multi-User Deployment)

Installs Voquill system-wide for all users on the machine. Requires administrator or `sudo` privileges.

**Windows (MSI):**
```powershell
irm https://voquill.org/install-system.ps1 | iex
```

**Linux (Package Manager — dnf/apt):**
```bash
curl -sf https://voquill.org/install.sh | sudo bash -s -- --system --yes
```

## Script Options

| Option (Linux) | Option (Windows) | Description |
|----------------|------------------|-------------|
| `--system` | `-System` | Install system-wide (requires admin / sudo) |
| `--version <tag>` | `-Version <tag>` | Install a specific release tag (e.g. `v1.5.0`) |
| `--channel <name>` | — | Release channel: `latest` or `stable` (default: `latest`) |
| `--yes` | — | Run non-interactively (skip confirmation prompts) |
| `--insecure-skip-verify` | `-InsecureSkipVerify` | Skip checksum verification (not recommended) |
| `--help` | `-Help` | Show script usage |

## Environment Overrides

| Variable | Description |
|----------|-------------|
| `VOQUILL_INSTALL_URL` | Full package URL override (skips GitHub API resolution) |
| `VOQUILL_CHECKSUM_URL` | Full checksum URL override |

## Manual Installation

Pre-built packages are available on the [Releases](https://github.com/jackbrumley/voquill/releases) page:

- `voquill-<version>-linux-x64.rpm` — Fedora / RHEL
- `voquill-<version>-linux-x64.deb` — Debian / Ubuntu
- `voquill-<version>-linux-x64.AppImage` — Cross-distro fallback
- `voquill-<version>-windows-x64-setup.exe` — Windows
- `voquill-<version>-windows-x64.msi` — Windows

```bash
# Fedora / RHEL
sudo dnf install ./voquill-*.rpm

# Debian / Ubuntu
sudo apt install ./voquill-*.deb

# AppImage (any Linux)
chmod +x voquill-*.AppImage
./voquill-*.AppImage
```