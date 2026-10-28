#!/usr/bin/env bash
# Voquill uninstaller for macOS. uninstall.sh hands off to this script on
# macOS. Must stay compatible with the bash 3.2 that ships with macOS.
set -euo pipefail

APP_PATH="/Applications/Voquill.app"
USER_DATA_DIR="${HOME}/.config/voquill-app"

usage() {
  cat <<'EOF_USAGE'
Voquill uninstall script for macOS

Usage:
  uninstall.sh [--purge-data] [--yes]

Default behavior:
  Removes /Applications/Voquill.app. Leaves user data (~/.config/voquill-app) in place.

Options:
  --purge-data  Also remove ~/.config/voquill-app (models, config, history, logs)
  --yes         Skip confirmation prompts
  -h, --help    Show this help message
EOF_USAGE
}

log() {
  printf "[voquill-uninstall] %s\n" "$*"
}

fail() {
  printf "[voquill-uninstall] ERROR: %s\n" "$*" >&2
  exit 1
}

purge_data=false
non_interactive=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --purge-data) purge_data=true ;;
    --yes) non_interactive=true ;;
    -h|--help)
      usage
      exit 0
      ;;
    *) fail "unknown option: $1" ;;
  esac
  shift
done

if [[ ! -t 0 ]]; then
  non_interactive=true
fi

log "This will remove ${APP_PATH}"
if [[ "$purge_data" == true ]]; then
  log "and all user data in ${USER_DATA_DIR}"
fi

if [[ "$non_interactive" == false ]]; then
  printf "Proceed? [y/N] "
  read -r answer
  if [[ "${answer:-}" != "y" && "${answer:-}" != "Y" ]]; then
    fail "uninstall cancelled"
  fi
fi

if pgrep -x voquill >/dev/null 2>&1; then
  log "Stopping running Voquill"
  osascript -e 'tell application "Voquill" to quit' >/dev/null 2>&1 || true
  sleep 2
  pkill -KILL -x voquill >/dev/null 2>&1 || true
fi

if [[ -d "$APP_PATH" ]]; then
  log "Removing ${APP_PATH}"
  if [[ -w "$(dirname "$APP_PATH")" ]]; then
    rm -rf "$APP_PATH"
  else
    sudo rm -rf "$APP_PATH"
  fi
else
  log "${APP_PATH} is not installed"
fi

if [[ "$purge_data" == true && -d "$USER_DATA_DIR" ]]; then
  log "Removing user data: ${USER_DATA_DIR}"
  rm -rf "${USER_DATA_DIR:?}"
fi

log "Uninstall complete"
log "Voquill's Microphone, Accessibility and Input Monitoring entries can be removed in System Settings → Privacy & Security."
