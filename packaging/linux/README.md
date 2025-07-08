# Voquill Linux Installation

Simple installation scripts for Voquill on Linux systems.

## Installation

```bash
cd packaging/linux
./install.sh
```

This installs Voquill to your home directory:
- Binary: `~/.local/bin/voquill`
- Icon: `~/.local/share/icons/hicolor/256x256/apps/voquill.png`
- Desktop file: `~/.local/share/applications/voquill.desktop`

The installer will remind you if you need to add `~/.local/bin` to your PATH.

## Uninstallation

```bash
cd packaging/linux
./uninstall.sh
```

## Requirements

- Linux system with bash
- The `voquill-linux-amd64` binary in `../../bin/`
- The icon file in `../../assets/icon256x256.png`

That's it! No sudo required, no configuration needed.
