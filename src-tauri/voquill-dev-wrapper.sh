#!/usr/bin/env bash
# Voquill Development Wrapper
# This script sets the Wayland app_id BEFORE launching the binary
# to ensure KDE Plasma's GlobalShortcuts portal correctly attributes the app.

export WAYLAND_APP_ID="org.voquill.foss"
export GDK_BACKEND="wayland"
export DESKTOP_ENTRY="org.voquill.foss"

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Tauri/Cargo appends "run" as the first argument; we should strip it
# It also appends "--no-default-features" etc if passed to cargo
# We want to pass everything after "run" to the binary, but actually 
# tauri calls "runner run <args>". 
# The actual binary we want to run is in target/debug/voquill

# The first argument from Tauri is usually the path to the binary
# If it's "run", we shift it (some versions of Tauri/Cargo do this)
if [ "$1" = "run" ]; then
    shift
fi

# Use the provided binary path if it exists, otherwise fallback to debug
TARGET_BIN="$1"
if [ -z "$TARGET_BIN" ] || [ ! -f "$TARGET_BIN" ]; then
    TARGET_BIN="$SCRIPT_DIR/target/debug/voquill"
else
    shift
fi

# Launch via systemd-run to break the terminal process tree link
# This ensures KDE sees the app_id as "org.voquill.foss" instead of "org.kde.konsole"
exec systemd-run --user --scope --unit=foss-voquill-dev \
    --setenv=WAYLAND_APP_ID="org.voquill.foss" \
    --setenv=GDK_BACKEND="wayland" \
    --setenv=DESKTOP_ENTRY="org.voquill.foss" \
    "$TARGET_BIN" "$@"
