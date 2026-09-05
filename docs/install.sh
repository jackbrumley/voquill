#!/usr/bin/env bash
set -euo pipefail

REPO="jackbrumley/voquill"
BIN_NAME="voquill"

usage() {
  cat <<'EOF'
Voquill bootstrap installer

Usage:
  install.sh [--appimage] [--system] [--version <tag>] [--channel latest|stable] [--yes] [--no-relaunch] [--insecure-skip-verify]

Default: system-wide install via apt/dnf (requires sudo/pkexec). Falls back to AppImage
if no supported package manager or root authentication is unavailable.

Options:
  --appimage              Install AppImage to ~/.local/bin (user-local, no sudo)
  --system                Force system-wide install via package manager (requires root)
  --clean                 Remove old packages and purge cached data (models, python-runner, debug)
                          before installing. Keeps config.json and history.db.
  --version <tag>         Install specific release tag (e.g. v1.5.0)
  --channel <name>        Release channel (default: latest)
  --yes                   Skip interactive confirmation prompts
  --no-relaunch           Do not launch Voquill after installation (launches by default)
  --relaunch              Explicitly launch Voquill after installation (default behavior)
  --insecure-skip-verify  Skip checksum verification (not recommended)
  -h, --help              Show this help message

Environment overrides:
  VOQUILL_INSTALL_URL     Full package URL override
  VOQUILL_NO_RELAUNCH     Set to 1 to disable auto-launching Voquill after install

Examples:
  curl -sf https://voquill.org/install.sh | bash
  curl -sf https://voquill.org/install.sh | bash -s -- --appimage
  curl -sf https://voquill.org/install.sh | sudo bash -s -- --yes --clean
EOF
}

LOG_DIR="${HOME}/.config/voquill-app/debug"
mkdir -p "$LOG_DIR" 2>/dev/null || true
LOG_FILE="${LOG_DIR}/update.log"

log() {
  local timestamp
  timestamp="$(date '+%Y-%m-%d %H:%M:%S.%3N' 2>/dev/null || date '+%Y-%m-%d %H:%M:%S')"
  printf "[voquill] %s\n" "$*"
  if [[ -n "${LOG_FILE:-}" && -d "$LOG_DIR" ]]; then
    printf "[%s] [voquill] %s\n" "$timestamp" "$*" >> "$LOG_FILE" 2>/dev/null || true
  fi
}

fail() {
  local timestamp
  timestamp="$(date '+%Y-%m-%d %H:%M:%S.%3N' 2>/dev/null || date '+%Y-%m-%d %H:%M:%S')"
  printf "[voquill] ERROR: %s\n" "$*" >&2
  if [[ -n "${LOG_FILE:-}" && -d "$LOG_DIR" ]]; then
    printf "[%s] [voquill] ERROR: %s\n" "$timestamp" "$*" >> "$LOG_FILE" 2>/dev/null || true
  fi
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

optional_cmd() {
  command -v "$1" >/dev/null 2>&1
}

require_safe_path() {
  local path="$1"
  [[ -n "$path" ]] || fail "refusing empty path"
  [[ "$path" != "/" ]] || fail "refusing root path"
}

run_as_root() {
  if [[ "$EUID" -eq 0 ]]; then
    "$@"
  elif optional_cmd sudo && sudo -n true 2>/dev/null; then
    sudo "$@"
  elif [[ -t 0 ]] && optional_cmd sudo; then
    sudo "$@"
  elif optional_cmd pkexec && [[ -n "${DISPLAY:-}" || -n "${WAYLAND_DISPLAY:-}" ]]; then
    pkexec "$@"
  elif optional_cmd sudo; then
    sudo "$@"
  else
    fail "Root permissions required but neither pkexec nor sudo is available"
  fi
}

acquire_root() {
  if [[ "$EUID" -eq 0 ]]; then
    return 0
  fi
  if optional_cmd sudo && sudo -n true 2>/dev/null; then
    return 0
  fi
  if [[ -t 0 ]]; then
    optional_cmd sudo || return 1
    sudo -v
  else
    if optional_cmd pkexec && [[ -n "${DISPLAY:-}" || -n "${WAYLAND_DISPLAY:-}" ]]; then
      return 0
    elif optional_cmd sudo; then
      sudo -n true 2>/dev/null || return 1
    else
      return 1
    fi
  fi
}

stop_running_process() {
  if optional_cmd pgrep && pgrep -x voquill >/dev/null 2>&1; then
    log "Stopping running Voquill process"
    if optional_cmd pkill; then
      pkill -TERM -x voquill >/dev/null 2>&1 || true
      sleep 1
      pkill -KILL -x voquill >/dev/null 2>&1 || true
    fi
  fi
}

resolve_arch() {
  local arch
  arch="$(uname -m)"
  case "$arch" in
    x86_64|amd64) printf "x64" ;;
    aarch64|arm64) printf "arm64" ;;
    *) fail "unsupported architecture: $arch" ;;
  esac
}

detect_package_manager() {
  if optional_cmd dnf; then
    printf "dnf"
  elif optional_cmd apt-get; then
    printf "apt"
  else
    printf "none"
  fi
}

resolve_package_ext() {
  local pm="$1"
  case "$pm" in
    dnf) printf "rpm" ;;
    apt) printf "deb" ;;
    *) printf "AppImage" ;;
  esac
}

fetch_latest_release_tag() {
  local api_url="https://api.github.com/repos/${REPO}/releases/latest"
  local tag
  tag="$(curl -fL --retry 3 --connect-timeout 10 -s "$api_url" | grep '"tag_name"' | head -n1 | sed 's/.*"tag_name": "\(.*\)",/\1/')"
  [[ -n "$tag" ]] || fail "could not determine latest release tag from GitHub API"
  printf "%s" "$tag"
}

uninstall_existing() {
  log "Checking for previous installations"
  local has_root=false
  if [[ "$EUID" -eq 0 ]] || (optional_cmd sudo && sudo -n true 2>/dev/null) || (optional_cmd pkexec && [[ -n "${DISPLAY:-}" || -n "${WAYLAND_DISPLAY:-}" ]]); then
    has_root=true
  fi

  # 1. Remove legacy package names via package manager (note: "voquill" is upgraded in-place by apt/dnf)
  local legacy_packages=("org.voquill.desktop" "org.voquill.app" "org.voquill.foss")
  for pkg in "${legacy_packages[@]}"; do
    if optional_cmd rpm && [[ "$has_root" == true ]]; then
      for rpm_name in "$pkg" "${pkg//./-}"; do
        if rpm -q "$rpm_name" >/dev/null 2>&1; then
          log "Removing legacy package: ${rpm_name}"
          run_as_root dnf remove -y "$rpm_name" 2>/dev/null || run_as_root rpm -e "$rpm_name" 2>/dev/null || true
          break
        fi
      done
    fi
    if optional_cmd dpkg && [[ "$has_root" == true ]] && dpkg -s "$pkg" 2>/dev/null | grep -q "Status: install ok installed"; then
      log "Removing legacy package: ${pkg}"
      run_as_root apt remove -y "$pkg" 2>/dev/null || run_as_root dpkg -r "$pkg" 2>/dev/null || true
    fi
  done

  # 2. Clean leftover binaries and desktop integration based on target install mode
  if [[ "$install_system" == true ]]; then
    # When installing system-wide, remove any user-local AppImage and desktop files that could shadow /usr/bin/voquill
    local user_binary="${HOME}/.local/bin/voquill"
    require_safe_path "$user_binary"
    if [[ -e "$user_binary" || -L "$user_binary" ]]; then
      log "Removing user-local binary: ${user_binary}"
      rm -f "$user_binary" 2>/dev/null || true
    fi

    local user_desktop_files=(
      "${HOME}/.local/share/applications/voquill.desktop"
      "${HOME}/.local/share/applications/org.voquill.desktop.desktop"
      "${HOME}/.local/share/applications/org.voquill.app.desktop"
    )
    for dfile in "${user_desktop_files[@]}"; do
      require_safe_path "$dfile"
      if [[ -e "$dfile" || -L "$dfile" ]]; then
        log "Removing user-local desktop entry: ${dfile}"
        rm -f "$dfile" 2>/dev/null || true
      fi
    done
  else
    # When installing user-local AppImage, remove old user-local binary and legacy entries
    local user_binary="${HOME}/.local/bin/voquill"
    require_safe_path "$user_binary"
    if [[ -e "$user_binary" || -L "$user_binary" ]]; then
      rm -f "$user_binary" 2>/dev/null || true
    fi
  fi
}

clean_user_data() {
  local config_dir="${HOME}/.config/voquill-app"
  if [[ ! -d "$config_dir" ]]; then
    log "No user data directory found, skipping cleanup"
    return
  fi

  log "Cleaning cached user data (keeping config.json and history.db)"

  for dir in "models" "python-runner" "debug"; do
    local target="${config_dir}/${dir}"
    if [[ -d "$target" ]]; then
      log "  Removing ${dir}/"
      rm -rf "$target"
    fi
  done

  log "User data cleaned"
}

install_system=false
install_appimage=false
non_interactive=false
skip_verify=false
clean=false
relaunch=true
if [[ "${VOQUILL_NO_RELAUNCH:-}" == "1" || "${VOQUILL_NO_RELAUNCH:-}" == "true" ]]; then
  relaunch=false
fi
version=""
channel="latest"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --system)
      install_system=true
      ;;
    --appimage)
      install_appimage=true
      ;;
    --clean)
      clean=true
      ;;
    --relaunch)
      relaunch=true
      ;;
    --no-relaunch)
      relaunch=false
      ;;
    --version)
      [[ $# -ge 2 ]] || fail "--version requires a value"
      version="$2"
      shift
      ;;
    --channel)
      [[ $# -ge 2 ]] || fail "--channel requires a value"
      channel="$2"
      shift
      ;;
    --yes)
      non_interactive=true
      ;;
    --insecure-skip-verify)
      skip_verify=true
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

need_cmd curl

if [[ ! -t 0 ]]; then
  non_interactive=true
fi

arch="$(resolve_arch)"
os="linux"

if [[ -z "$version" ]]; then
  if [[ "$channel" == "latest" || "$channel" == "stable" ]]; then
    version="latest"
  else
    fail "unsupported channel: $channel"
  fi
fi

# Resolve install mode: default to system if package manager + root/sudo/polkit available
if [[ "$install_appimage" == true ]]; then
  install_system=false
elif [[ "$install_system" == true ]]; then
  pm="$(detect_package_manager)"
  if [[ "$pm" == "none" ]]; then
    fail "No supported package manager found (dnf/apt). Use --appimage for user-local install."
  fi
  if [[ "$EUID" -ne 0 ]]; then
    log "System install requested; root authorization required"
    acquire_root || fail "failed to acquire root permission for system install"
  fi
else
  pm="$(detect_package_manager)"
  if [[ "$pm" != "none" ]]; then
    if [[ "$EUID" -eq 0 ]]; then
      install_system=true
    else
      has_system_install=false
      if [[ -x "/usr/bin/voquill" || -x "/usr/local/bin/voquill" ]]; then
        has_system_install=true
      fi

      log "Package manager '${pm}' detected. Acquiring root permissions..."
      if acquire_root; then
        install_system=true
      elif [[ "$has_system_install" == true ]]; then
        fail "Root permissions required to update existing system package in /usr/bin. Update aborted."
      else
        log "Root authentication unavailable. Falling back to AppImage."
        install_system=false
      fi
    fi
  else
    log "No supported package manager found. Falling back to AppImage."
    install_system=false
  fi
fi

if [[ -n "${VOQUILL_INSTALL_URL:-}" ]]; then
  asset_url="$VOQUILL_INSTALL_URL"
  package_ext="${asset_url##*.}"
else
  if [[ "$version" == "latest" ]]; then
    release_tag="$(fetch_latest_release_tag)"
  else
    release_tag="$version"
  fi

  if [[ "$install_system" == true ]]; then
    package_ext="$(resolve_package_ext "$pm")"
  else
    package_ext="AppImage"
  fi

  gh_asset_name="${BIN_NAME}-${release_tag#v}-${os}-${arch}.${package_ext}"
  log "Resolving asset: ${gh_asset_name}"
  asset_url="https://github.com/${REPO}/releases/download/${release_tag}/${gh_asset_name}"
fi

log "Preparing Voquill install"
log "Release: ${release_tag:-${version}}"

if [[ "$install_system" == true ]]; then
  log "Install mode: system-wide (${pm})"
  log "Package: ${package_ext}"
else
  log "Install mode: user (AppImage)"
fi

if [[ "$non_interactive" == false ]]; then
  printf "Proceed with installation? [y/N] "
  read -r answer
  if [[ "${answer:-}" != "y" && "${answer:-}" != "Y" ]]; then
    fail "installation cancelled"
  fi
else
  log "Non-interactive mode detected, continuing automatically"
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

package_path="${tmp_dir}/${BIN_NAME}.${package_ext:-AppImage}"

log "Downloading release artifact"
log "  ${asset_url}"
if ! curl -fL --retry 3 --connect-timeout 15 -o "$package_path" "$asset_url"; then
  fail "download failed. No release artifact was found for this target yet.\nTry local development run:\n  git clone https://github.com/${REPO}\n  cd voquill\n  npm install\n  npm run tauri dev"
fi

verify_checksum() {
  local file="$1"
  local asset_name="$2"
  local api_url="https://api.github.com/repos/${REPO}/releases/tags/${release_tag}"

  if ! optional_cmd python3; then
    log "WARNING: python3 not found, skipping checksum verification"
    return
  fi

  local digest
  digest="$(curl -sfL --retry 3 --connect-timeout 10 -H "Accept: application/vnd.github+json" "$api_url" | python3 -c "
import json,sys
data=json.load(sys.stdin)
for asset in data.get('assets', []):
    if asset['name'] == '$asset_name':
        print(asset.get('digest', ''))
" 2>/dev/null)" || digest=""
  if [[ -z "$digest" ]]; then
    log "WARNING: could not fetch checksum from GitHub API, skipping verification"
    return
  fi
  local expected="${digest#sha256:}"
  local actual
  actual="$(sha256sum "$file" | awk '{print $1}')"
  if [[ "$expected" != "$actual" ]]; then
    fail "checksum mismatch (expected $expected, got $actual)"
  fi
  log "Checksum verified"
}

if [[ "$skip_verify" == true ]]; then
  log "WARNING: checksum verification disabled"
elif [[ -n "${release_tag:-}" ]]; then
  need_cmd sha256sum
  verify_checksum "$package_path" "$gh_asset_name"
fi

# Stop any running process and remove previous installation before deploying new version
stop_running_process
uninstall_existing

if [[ "$clean" == true ]]; then
  log "Clean mode enabled"
  clean_user_data
fi

if [[ "$install_system" == true ]]; then
  log "Installing with ${pm} (system privileges required)"

  if [[ "$pm" == "dnf" ]]; then
    run_as_root dnf install -y "$package_path"
  elif [[ "$pm" == "apt" ]]; then
    run_as_root apt install -y "$package_path"
  fi

  log "System installation complete via ${pm}"
else
  install_dir="${HOME}/.local/bin"
  desktop_dir="${HOME}/.local/share/applications"
  icon_dir="${HOME}/.local/share/icons/hicolor/scalable/apps"

  log "Installing AppImage to ${install_dir}"
  install -d "$install_dir" "$desktop_dir" "$icon_dir"

  appimage_path="${install_dir}/${BIN_NAME}"
  install -m 0755 "$package_path" "$appimage_path"

  if [[ -f "${appimage_path}" ]]; then
    log "AppImage installed at ${appimage_path}"

    desktop_file="${desktop_dir}/voquill.desktop"
    log "Creating desktop launcher"
    cat > "$desktop_file" <<EOF
[Desktop Entry]
Name=Voquill
Comment=Push-to-talk dictation app with local transcription
Exec=${appimage_path}
Terminal=false
Type=Application
Icon=${BIN_NAME}
StartupWMClass=voquill
Categories=Utility;Office;AudioVideo;
StartupNotify=true
EOF

    icon_file="${icon_dir}/voquill.svg"
    log "Embedding icon from AppImage"
    extraction_dir="${tmp_dir}/appimage-extract"
    mkdir -p "$extraction_dir"
    if (cd "$tmp_dir" && "${appimage_path}" --appimage-extract >/dev/null 2>&1); then
      find "$extraction_dir" -name "*.png" -o -name "*.svg" 2>/dev/null | head -n1 | while read -r ico; do
        cp "$ico" "$icon_file" 2>/dev/null || true
      done
      rm -rf "$extraction_dir"
    fi
  fi
fi

if optional_cmd gtk-update-icon-cache; then
  if [[ "$install_system" == true ]]; then
    if [[ "$EUID" -eq 0 ]] || (optional_cmd sudo && sudo -n true 2>/dev/null); then
      run_as_root gtk-update-icon-cache -f -t -q /usr/share/icons/hicolor >/dev/null 2>&1 || true
    fi
  else
    gtk-update-icon-cache -f -t -q "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
  fi
fi

if optional_cmd update-desktop-database; then
  if [[ "$install_system" == true ]]; then
    if [[ "$EUID" -eq 0 ]] || (optional_cmd sudo && sudo -n true 2>/dev/null); then
      run_as_root update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
    fi
  else
    update-desktop-database "${HOME}/.local/share/applications" >/dev/null 2>&1 || true
  fi
fi

if optional_cmd kbuildsycoca6; then
  log "Rebuilding KDE menu cache"
  kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
elif optional_cmd kbuildsycoca5; then
  log "Rebuilding KDE menu cache"
  kbuildsycoca5 --noincremental >/dev/null 2>&1 || true
fi

if [[ "$install_system" == false ]] && [[ ":$PATH:" != *":${install_dir}:"* ]]; then
  log "${install_dir} is not currently in PATH"
  log "Add this line to your shell profile and restart terminal:"
  log "  export PATH=\"${install_dir}:\$PATH\""
fi

log "Installation complete"
log "Run: ${BIN_NAME}"

if [[ "$install_system" == true ]]; then
  log "Or launch from your application menu"
fi

if [[ "$relaunch" == true ]]; then
  log "Relaunching Voquill..."
  if [[ "$install_system" == true ]]; then
    if optional_cmd voquill; then
      nohup voquill >/dev/null 2>&1 &
    elif [[ -x "/usr/bin/voquill" ]]; then
      nohup /usr/bin/voquill >/dev/null 2>&1 &
    fi
  else
    if [[ -n "${appimage_path:-}" && -x "${appimage_path}" ]]; then
      nohup "${appimage_path}" >/dev/null 2>&1 &
    elif [[ -x "${HOME}/.local/bin/voquill" ]]; then
      nohup "${HOME}/.local/bin/voquill" >/dev/null 2>&1 &
    fi
  fi
fi