#!/usr/bin/env bash
# Voquill Development Wrapper
# This script sets the Wayland app_id BEFORE launching the binary
# to ensure KDE Plasma's GlobalShortcuts portal correctly attributes the app.

export WAYLAND_APP_ID="com.voquill.voquill"
export GDK_BACKEND="wayland"
export DESKTOP_ENTRY="com.voquill.voquill"

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Tauri/Cargo appends "run" as the first argument; we should strip it
# It also appends "--no-default-features" etc if passed to cargo
# We want to pass everything after "run" to the binary, but actually 
# tauri calls "runner run <args>". 
# The actual binary we want to run is in target/debug/voquill

# Strip "run" if it's the first arg
if [ "$1" = "run" ]; then
    shift
fi

# Launch via systemd-run to break the terminal process tree link
# This ensures KDE sees the app_id as "com.voquill.voquill" instead of "org.kde.konsole"
exec systemd-run --user --scope --unit=voquill-dev \
    --setenv=WAYLAND_APP_ID="com.voquill.voquill" \
    --setenv=GDK_BACKEND="wayland" \
    --setenv=DESKTOP_ENTRY="com.voquill.voquill" \
    "$SCRIPT_DIR/target/debug/voquill" "$@"
