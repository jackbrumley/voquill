#!/bin/bash

# Voquill Linux Uninstallation Script

set -e

# Uninstall from user's local directory
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

# Remove binary
if [[ -f "$INSTALL_PREFIX/bin/voquill" ]]; then
    print_status "Removing binary..."
    rm -f "$INSTALL_PREFIX/bin/voquill"
else
    print_warning "Binary not found"
fi

# Remove icon
if [[ -f "$INSTALL_PREFIX/share/icons/hicolor/256x256/apps/voquill.png" ]]; then
    print_status "Removing icon..."
    rm -f "$INSTALL_PREFIX/share/icons/hicolor/256x256/apps/voquill.png"
else
    print_warning "Icon not found"
fi

# Remove desktop file
if [[ -f "$DESKTOP_FILE_DIR/voquill.desktop" ]]; then
    print_status "Removing desktop file..."
    rm -f "$DESKTOP_FILE_DIR/voquill.desktop"
else
    print_warning "Desktop file not found"
fi

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

print_status "Uninstallation complete!"
