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

## Quick Install (System Package)

Detects your package manager (`dnf` on Fedora/RHEL, `apt` on Debian/Ubuntu), downloads the matching
package from the latest GitHub release, and installs it system-wide.

```bash
curl -sf https://voquill.org/install.sh | sudo bash -s -- --system --yes
```

## User Install (AppImage)

Installs the AppImage to `~/.local/bin/voquill` and creates a desktop launcher. No `sudo` needed.

```bash
curl -sf https://voquill.org/install.sh | bash
```

If `~/.local/bin` is not in your PATH, the script will print instructions to add it.

## Script Options

| Option | Description |
|--------|-------------|
| `--system` | Install system-wide via package manager (requires sudo) |
| `--version <tag>` | Install a specific release tag (e.g. `v1.5.0`) |
| `--channel <name>` | Release channel: `latest` or `stable` (default: `latest`) |
| `--yes` | Run non-interactively (skip confirmation prompts) |
| `--insecure-skip-verify` | Skip checksum verification (not recommended) |
| `--help` | Show script usage |

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