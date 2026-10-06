---
layout: default
title: Voquill Install Guide
description: One-command install of Voquill on Linux and Windows. System package, AppImage, or PowerShell.
canonical: https://voquill.org/INSTALL.html
body_class: centered-page
container_class: home-container
---

# Voquill Install Guide

Voquill supports a one-command install flow from the website, matching the uninstall UX. Every installation automatically stops running instances and cleanly replaces previous versions or legacy shortcuts before deploying the latest build, while preserving your settings and models.

## Linux Installation

### Native Package (.deb / .rpm) — Recommended
Auto-detects `apt` or `dnf` and installs system-wide (prompts for sudo if required):
```bash
curl -sf https://voquill.org/install.sh | bash
```

### AppImage (User-Local / Standalone)
Installs AppImage to `~/.local/bin` without requiring sudo or admin privileges:
```bash
curl -sf https://voquill.org/install.sh | bash -s -- --appimage
```

## Windows Installation

Open **PowerShell** or **Windows Terminal**, paste the command, and press <kbd>Enter</kbd>:

### User Install (Recommended — No Admin Required)
Installs Voquill in the current user context:
```powershell
irm https://voquill.org/install.ps1 | iex
```

### System Install (MSI — All Users)
Installs Voquill system-wide for all users on the machine (requires administrator privileges):
```powershell
irm https://voquill.org/install-system.ps1 | iex
```

## Script Options

| Option (Linux) | Option (Windows) | Description |
|----------------|------------------|-------------|
| `--appimage` | — | Install AppImage to ~/.local/bin (no sudo) |
| `--system` | `-System` | Force system-wide install (requires admin / sudo) |
| `--clean` | `-Clean` | Clean cached models/data before installing (keeps config and history) |
| `--version <tag>` | `-Version <tag>` | Install a specific release tag (e.g. `v1.5.0`) |
| `--channel <name>` | — | Release channel: `latest` or `stable` (default: `latest`) |
| `--yes` | — | Run non-interactively (skip confirmation prompts) |
| `--relaunch` | `-Relaunch` | Launch Voquill after successful installation |
| `--insecure-skip-verify` | `-InsecureSkipVerify` | Skip checksum verification (not recommended) |
| `--help` | `-Help` | Show script usage |

## Environment Overrides

| Variable | Description |
|----------|-------------|
| `VOQUILL_INSTALL_URL` | Full package URL override (skips GitHub API resolution) |

## Manual Installation

Pre-built packages are available on the [Releases](https://github.com/jackbrumley/voquill/releases) page:

- `voquill-<version>-linux-x64.rpm` — Fedora / RHEL
- `voquill-<version>-linux-x64.deb` — Debian / Ubuntu
- `voquill-<version>-linux-x64.AppImage` — Cross-distro fallback
- `voquill-<version>-windows-x64-setup.exe` — Windows
- `voquill-<version>-windows-x64.msi` — Windows

Linux packages require glibc 2.35 or newer (Ubuntu 22.04, Linux Mint 21, Debian 12, current Fedora, or later). Only x64 builds are published; ARM64 is not available yet.

Each file's SHA-256 checksum is shown next to it on the release page (GitHub generates it on upload); the install scripts verify downloads against it automatically.

```bash
# Fedora / RHEL
sudo dnf install ./voquill-*.rpm

# Debian / Ubuntu
sudo apt install ./voquill-*.deb

# AppImage (any Linux)
chmod +x voquill-*.AppImage
./voquill-*.AppImage
```