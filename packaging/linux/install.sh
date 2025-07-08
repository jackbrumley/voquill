#!/bin/bash

# Voquill Linux Installation Script

set -e

# Install to user's local directory
INSTALL_PREFIX="$HOME/.local"
DESKTOP_FILE_DIR="$HOME/.local/share/applications"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Create directories
print_status "Creating directories..."
mkdir -p "$INSTALL_PREFIX/bin"
mkdir -p "$INSTALL_PREFIX/share/icons/hicolor/256x256/apps"
mkdir -p "$DESKTOP_FILE_DIR"

# Copy binary
print_status "Installing binary to $INSTALL_PREFIX/bin/voquill..."
cp "../../bin/voquill-linux-amd64" "$INSTALL_PREFIX/bin/voquill"
chmod +x "$INSTALL_PREFIX/bin/voquill"

# Copy icon
print_status "Installing icon..."
cp "../../assets/icon256x256.png" "$INSTALL_PREFIX/share/icons/hicolor/256x256/apps/voquill.png"

# Process desktop file template
print_status "Installing desktop file..."
sed "s|{{INSTALL_PREFIX}}|$INSTALL_PREFIX|g" voquill.desktop.template > "$DESKTOP_FILE_DIR/voquill.desktop"

# Update desktop database if available
if command -v update-desktop-database >/dev/null 2>&1; then
    print_status "Updating desktop database..."
    update-desktop-database "$DESKTOP_FILE_DIR" 2>/dev/null || true
fi

# Update icon cache if available
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    print_status "Updating icon cache..."
    gtk-update-icon-cache -t "$INSTALL_PREFIX/share/icons/hicolor" 2>/dev/null || true
fi

print_status "Installation complete!"
print_status "Voquill has been installed to: $INSTALL_PREFIX/bin/voquill"

# Add to PATH reminder if needed
if [[ ":$PATH:" != *":$INSTALL_PREFIX/bin:"* ]]; then
    print_warning "Add $INSTALL_PREFIX/bin to your PATH by running:"
    print_warning "  echo 'export PATH=\"$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
    print_warning "  source ~/.bashrc"
fi
