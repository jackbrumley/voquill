#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Voquill uninstall script

Usage:
  uninstall.sh [--purge-data] [--yes]

Default behavior:
  Removes Voquill package (via dnf or apt) and cleans desktop integration.
  Leaves user data (~/.config/voquill-app) in place.

Options:
  --purge-data  Also remove ~/.config/voquill-app (models, config, history, logs)
  --yes         Skip confirmation prompts
  -h, --help    Show this help message

Examples:
  curl -sf https://voquill.org/uninstall.sh | bash
  curl -sf https://voquill.org/uninstall.sh | bash -s -- --purge-data --yes
EOF
}

log() {
  printf "[voquill-uninstall] %s\n" "$*"
}

fail() {
  printf "[voquill-uninstall] ERROR: %s\n" "$*" >&2
  exit 1
}

optional_cmd() {
  command -v "$1" >/dev/null 2>&1
}

run_as_root() {
  if [[ "$EUID" -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

require_safe_path() {
  local path="$1"
  [[ -n "$path" ]] || fail "refusing empty path"
  [[ "$path" != "/" ]] || fail "refusing root path"
}

non_interactive=false
purge_data=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --purge-data)
      purge_data=true
      ;;
    --yes)
      non_interactive=true
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "unknown option: $1"
      ;;
  esac
  shift
done

if [[ ! -t 0 ]]; then
  non_interactive=true
fi

# Known package names (current + legacy)
PACKAGE_NAMES=("voquill" "org.voquill.app" "org.voquill.foss")

# Known binary locations
BINARY_PATHS=(
  "/usr/bin/voquill"
  "/usr/local/bin/voquill"
  "${HOME}/.local/bin/voquill"
)

# Known desktop integration files
DESKTOP_FILES=(
  "/usr/share/applications/voquill.desktop"
  "/usr/share/applications/org.voquill.app.desktop"
  "${HOME}/.local/share/applications/voquill.desktop"
  "${HOME}/.local/share/applications/org.voquill.app.desktop"
)

ICON_DIRS=(
  "/usr/share/icons/hicolor/scalable/apps"
  "${HOME}/.local/share/icons/hicolor/scalable/apps"
)

METAINFO_FILES=(
  "/usr/share/metainfo/org.voquill.app.metainfo.xml"
  "/usr/share/appdata/org.voquill.app.metainfo.xml"
)

# User data directory
USER_DATA_DIR="${HOME}/.config/voquill-app"

system_changes_required=false
package_installed=false

# Check if any package is installed
for pkg in "${PACKAGE_NAMES[@]}"; do
  if optional_cmd rpm; then
    # Tauri converts dots to dashes in RPM package names, try both
    for rpm_name in "$pkg" "${pkg//./-}"; do
      if rpm -q "$rpm_name" >/dev/null 2>&1; then
        package_installed=true
        system_changes_required=true
        break
      fi
    done
  fi
  if [[ "$package_installed" != true ]] && optional_cmd dpkg && dpkg -l "$pkg" >/dev/null 2>&1; then
    package_installed=true
    system_changes_required=true
  fi
done

# Check for system-level files
for path in "${BINARY_PATHS[@]}" "/usr/bin/voquill" "/usr/local/bin/voquill"; do
  [[ -e "$path" || -L "$path" ]] && system_changes_required=true
done
for path in "${DESKTOP_FILES[@]}"; do
  [[ "$path" == /usr/share/* && ( -e "$path" || -L "$path" ) ]] && system_changes_required=true
done
for path in "${METAINFO_FILES[@]}"; do
  [[ -e "$path" || -L "$path" ]] && system_changes_required=true
done

if [[ "$system_changes_required" == true && "$EUID" -ne 0 ]]; then
  optional_cmd sudo || fail "system cleanup requires sudo, but sudo was not found"
fi

log "Preparing Voquill uninstall"

if [[ "$non_interactive" == false ]]; then
  printf "Proceed with uninstall? [y/N] "
  read -r answer
  if [[ "${answer:-}" != "y" && "${answer:-}" != "Y" ]]; then
    fail "uninstall cancelled"
  fi
else
  log "Non-interactive mode detected"
fi

# Kill running process if found
if optional_cmd pgrep && pgrep -x voquill >/dev/null 2>&1; then
  if optional_cmd pkill; then
    log "Closing running Voquill process"
    pkill -TERM -x voquill >/dev/null 2>&1 || true
    sleep 1
    pkill -KILL -x voquill >/dev/null 2>&1 || true
  else
    log "Voquill appears to be running, but pkill is unavailable"
  fi
fi

# Acquire sudo if needed
if [[ "$system_changes_required" == true && "$EUID" -ne 0 ]]; then
  log "System-level cleanup detected; sudo may prompt for your password"
  sudo -v || fail "failed to acquire sudo permission for system cleanup"
fi

# Remove via package manager
for pkg in "${PACKAGE_NAMES[@]}"; do
  if optional_cmd rpm; then
    # Tauri converts dots to dashes in RPM package names, try both
    for rpm_name in "$pkg" "${pkg//./-}"; do
      if rpm -q "$rpm_name" >/dev/null 2>&1; then
        log "Removing RPM package: ${rpm_name}"
        run_as_root dnf remove -y "$rpm_name" 2>/dev/null || run_as_root rpm -e "$rpm_name" 2>/dev/null || log "Package removal may have already been handled"
        break
      fi
    done
  fi
  if optional_cmd dpkg && dpkg -l "$pkg" >/dev/null 2>&1; then
    log "Removing DEB package: ${pkg}"
    run_as_root apt remove -y "$pkg" 2>/dev/null || run_as_root dpkg -r "$pkg" 2>/dev/null || log "Package removal may have already been handled"
  fi
done

# Clean up any leftover binaries
for path in "${BINARY_PATHS[@]}"; do
  require_safe_path "$path"
  if [[ -e "$path" || -L "$path" ]]; then
    log "Removing binary: ${path}"
    if [[ "$path" == /usr/* ]]; then
      run_as_root rm -f "$path"
    else
      rm -f "$path"
    fi
  fi
done

# Clean up desktop files
for path in "${DESKTOP_FILES[@]}"; do
  require_safe_path "$path"
  if [[ -e "$path" || -L "$path" ]]; then
    log "Removing desktop file: ${path}"
    if [[ "$path" == /usr/share/* ]]; then
      run_as_root rm -f "$path"
    else
      rm -f "$path"
    fi
  fi
done

# Clean up icons
for icon_dir in "${ICON_DIRS[@]}"; do
  require_safe_path "$icon_dir"
  icon_path="${icon_dir}/voquill.svg"
  legacy_icon_path="${icon_dir}/org.voquill.app.svg"
  for ico in "$icon_path" "$legacy_icon_path"; do
    if [[ -e "$ico" || -L "$ico" ]]; then
      log "Removing icon: ${ico}"
      if [[ "$ico" == /usr/share/* ]]; then
        run_as_root rm -f "$ico"
      else
        rm -f "$ico"
      fi
    fi
  done
done

# Clean up metainfo
for path in "${METAINFO_FILES[@]}"; do
  require_safe_path "$path"
  if [[ -e "$path" || -L "$path" ]]; then
    log "Removing metainfo: ${path}"
    run_as_root rm -f "$path"
  fi
done

# Purge user data if requested
if [[ "$purge_data" == true ]]; then
  require_safe_path "$USER_DATA_DIR"
  if [[ -d "$USER_DATA_DIR" ]]; then
    log "Removing user data: ${USER_DATA_DIR}"
    rm -rf "$USER_DATA_DIR"
  fi
  legacy_dir="${HOME}/.config/foss-voquill"
  if [[ -d "$legacy_dir" ]]; then
    log "Removing legacy data: ${legacy_dir}"
    rm -rf "$legacy_dir"
  fi
else
  if [[ -d "$USER_DATA_DIR" ]]; then
    log "User data left intact at ${USER_DATA_DIR}"
    log "To remove it manually: rm -rf ${USER_DATA_DIR}"
  fi
fi

# Refresh system caches
if optional_cmd update-desktop-database; then
  if [[ -d "/usr/share/applications" ]]; then
    log "Refreshing system desktop database"
    run_as_root update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
  fi
  if [[ -d "${HOME}/.local/share/applications" ]]; then
    log "Refreshing user desktop database"
    update-desktop-database "${HOME}/.local/share/applications" >/dev/null 2>&1 || true
  fi
fi

if optional_cmd gtk-update-icon-cache; then
  if [[ -d "/usr/share/icons/hicolor" ]]; then
    log "Refreshing system icon cache"
    run_as_root gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
  fi
  if [[ -d "${HOME}/.local/share/icons/hicolor" ]]; then
    log "Refreshing user icon cache"
    gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
  fi
fi

# Rebuild KDE's menu database so removed .desktop entries disappear from the
# application launcher / start menu immediately.
if optional_cmd kbuildsycoca6; then
  log "Rebuilding KDE menu cache"
  kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
elif optional_cmd kbuildsycoca5; then
  log "Rebuilding KDE menu cache"
  kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
fi

log "Uninstall complete"
if [[ "$purge_data" == true ]]; then
  log "Voquill and all user data were removed."
else
  log "Voquill was removed. User data (~/.config/voquill-app) was left intact."
  log "Re-run with --purge-data to remove it as well."
fi