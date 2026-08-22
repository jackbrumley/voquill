---
layout: default
title: Voquill Uninstall Guide
description: One-command uninstall of Voquill on Linux and Windows. Safe default or full purge.
canonical: https://voquill.org/UNINSTALL.html
body_class: centered-page
container_class: home-container
---

# Voquill Uninstall Guide

Voquill supports a one-command uninstall flow from the website, matching the install UX.

## Quick Uninstall (Safe Default)

Removes the Voquill package (via `dnf` or `apt`) and cleans up desktop integration files.
Leaves your user data (`~/.config/voquill-app`) intact so you don't lose models or settings.

```bash
curl -sf https://voquill.org/uninstall.sh | bash
```

## Full Purge (Clean Slate)

Removes Voquill and wipes all user data — models, config, history, logs, and the entire
`~/.config/voquill-app` directory.

```bash
curl -sf https://voquill.org/uninstall.sh | bash -s -- --purge-data --yes
```

## Script Options

| Option | Description |
|--------|-------------|
| `--purge-data` | Also remove `~/.config/voquill-app` (models, config, history, logs) |
| `--yes` | Run non-interactively (skip confirmation prompts) |
| `--help` | Show script usage |

## What Gets Removed

| Location | Always | With `--purge-data` |
|----------|--------|---------------------|
| System package (`voquill`, `org.voquill.desktop`, `org.voquill.app`, `org.voquill.foss`) | Yes | Yes |
| Binary (`/usr/bin/voquill`, `~/.local/bin/voquill`) | Yes | Yes |
| Desktop file (`*.desktop`) | Yes | Yes |
| Icons (`/usr/share/icons/hicolor/*/apps/voquill.*`) | Yes | Yes |
| Metainfo (`/usr/share/metainfo/org.voquill.desktop.metainfo.xml`) | Yes | Yes |
| User data (`~/.config/voquill-app`) | No | Yes |
| Legacy data (`~/.config/foss-voquill`) | No | Yes |

## Manual Uninstall

```bash
# RPM (Fedora / RHEL)
sudo dnf remove voquill

# DEB (Debian / Ubuntu)
sudo apt remove voquill

# Remove user data (optional)
rm -rf ~/.config/voquill-app
```