# Voquill Flatpak

This directory contains the Flatpak packaging configuration for Voquill, enabling installation on immutable Linux distributions like Bazzite, Fedora Silverblue, and others.

## Files

- `com.voquill.voquill.yml` - Main Flatpak manifest
- `com.voquill.voquill.desktop` - Desktop entry file
- `com.voquill.voquill.metainfo.xml` - AppStream metadata
- `README.md` - This file

## Building Locally

### Prerequisites

Install Flatpak and flatpak-builder:

```bash
# Fedora/RHEL
sudo dnf install flatpak flatpak-builder

# Ubuntu/Debian
sudo apt install flatpak flatpak-builder

# Arch Linux
sudo pacman -S flatpak flatpak-builder
```

Add Flathub repository:
```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
```

Install the required runtime:
```bash
flatpak install flathub org.freedesktop.Platform//23.08 org.freedesktop.Sdk//23.08
```

### Build Process

1. **Update SHA256 hash** in `com.voquill.voquill.yml`:
   ```bash
   # Get the SHA256 of the latest Linux binary
   curl -sL https://github.com/jackbrumley/voquill/releases/download/v1.0.2/voquill-linux-x86_64 | sha256sum
   ```

2. **Build the Flatpak**:
   ```bash
   flatpak-builder build-dir com.voquill.voquill.yml --force-clean
   ```

3. **Install locally**:
   ```bash
   flatpak-builder --user --install --force-clean build-dir com.voquill.voquill.yml
   ```

4. **Run the app**:
   ```bash
   flatpak run com.voquill.voquill
   ```

## Permissions

The Flatpak includes these permissions for full functionality:

- **GUI Access**: Wayland and X11 support
- **Audio**: Microphone access for voice input
- **Network**: OpenAI API communication
- **Global Shortcuts**: System-wide hotkey support
- **Input Simulation**: Typing into other applications
- **Configuration**: Access to store settings

## Installation

To install the Flatpak from GitHub releases:

```bash
# First install the required runtime:
flatpak install flathub org.freedesktop.Platform//23.08

# Then install the app:
flatpak install --user voquill.flatpak
```

## Testing on Immutable Distros

### Bazzite
```bash
# Install required runtime first:
flatpak install flathub org.freedesktop.Platform//23.08
# Then install the app:
flatpak install --user voquill.flatpak

# Or install from Flathub (once published)
flatpak install flathub com.voquill.voquill
```

### Fedora Silverblue
```bash
# Same commands as Bazzite
flatpak install flathub org.freedesktop.Platform//23.08
flatpak install --user voquill.flatpak
```

## Submitting to Flathub

1. Fork the [Flathub repository](https://github.com/flathub/flathub)
2. Create a new repository: `com.voquill.voquill`
3. Submit the manifest files
4. Follow the Flathub review process

## Troubleshooting

### Global Hotkeys Not Working
- Ensure your desktop environment supports the FreeDesktop portal for global shortcuts
- On KDE, make sure KGlobalAccel is running
- On GNOME, ensure the Shell portal is available

### Microphone Access Issues
- Check that PulseAudio/PipeWire is properly configured
- Verify microphone permissions in your desktop environment

### Typing Not Working
- Ensure the InputCapture portal is available
- Some desktop environments may require additional permissions

## Development

To modify the Flatpak:

1. Edit the manifest file (`com.voquill.voquill.yml`)
2. Update the SHA256 hash if using a new binary
3. Rebuild and test locally
4. Submit changes via pull request
