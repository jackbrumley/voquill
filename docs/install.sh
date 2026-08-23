#!/usr/bin/env bash
set -euo pipefail

REPO="jackbrumley/voquill"
BIN_NAME="voquill"

usage() {
  cat <<'EOF'
Voquill bootstrap installer

Usage:
  install.sh [--appimage] [--system] [--version <tag>] [--channel latest|stable] [--yes] [--insecure-skip-verify]

Default: system-wide install via apt/dnf (requires sudo). Falls back to AppImage
if no supported package manager or sudo is unavailable.

Options:
  --appimage              Install AppImage to ~/.local/bin (user-local, no sudo)
  --system                Force system-wide install via package manager (requires sudo)
  --clean                 Remove old packages and purge cached data (models, python-runner, debug)
                          before installing. Keeps config.json and history.db.
  --version <tag>         Install specific release tag (e.g. v1.5.0)
  --channel <name>        Release channel (default: latest)
  --yes                   Skip interactive confirmation prompts
  --insecure-skip-verify  Skip checksum verification (not recommended)
  -h, --help              Show this help message

Environment overrides:
  VOQUILL_INSTALL_URL     Full package URL override

Examples:
  curl -sf https://voquill.org/install.sh | bash
  curl -sf https://voquill.org/install.sh | bash -s -- --appimage
  curl -sf https://voquill.org/install.sh | sudo bash -s -- --yes --clean
EOF
}

log() {
  printf "[voquill] %s\n" "$*"
}

fail() {
  printf "[voquill] ERROR: %s\n" "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
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

acquire_root() {
  if [[ "$EUID" -ne 0 ]]; then
    optional_cmd sudo || return 1
    sudo -v
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

clean_old_packages() {
  local pm="$1"
  local removed=false
  for pkg in "voquill" "org.voquill.desktop" "org.voquill.app" "org.voquill.foss"; do
    if [[ "$pm" == "dnf" ]]; then
      # Tauri converts dots to dashes in RPM package names, try both
      for rpm_name in "$pkg" "${pkg//./-}"; do
        if rpm -q "$rpm_name" >/dev/null 2>&1; then
          log "Removing old package: ${rpm_name}"
          run_as_root dnf remove -y "$rpm_name" 2>/dev/null || true
          removed=true
          break
        fi
      done
    elif [[ "$pm" == "apt" ]] && dpkg -l "$pkg" >/dev/null 2>&1; then
      log "Removing old package: ${pkg}"
      run_as_root apt remove -y "$pkg" 2>/dev/null || true
      removed=true
    fi
  done
  if [[ "$removed" == true ]]; then
    log "Old packages removed"
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

# Resolve install mode: default to system if package manager + root/sudo available
if [[ "$install_appimage" == true ]]; then
  install_system=false
elif [[ "$install_system" == true ]]; then
  pm="$(detect_package_manager)"
  if [[ "$pm" == "none" ]]; then
    fail "No supported package manager found (dnf/apt). Use --appimage for user-local install."
  fi
  if [[ "$EUID" -ne 0 ]]; then
    log "System install requested; sudo may prompt for your password"
    acquire_root || fail "failed to acquire sudo permission for system install"
  fi
else
  pm="$(detect_package_manager)"
  if [[ "$pm" != "none" ]]; then
    if [[ "$EUID" -eq 0 ]]; then
      install_system=true
    else
      log "Package manager '${pm}' detected. Sudo may prompt for your password:"
      if acquire_root; then
        install_system=true
      else
        log "Sudo authentication unavailable. Falling back to AppImage."
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

if [[ "$clean" == true ]]; then
  log "Clean mode enabled"
  if [[ "$install_system" == true ]]; then
    acquire_root || fail "root/sudo permission required for --clean with --system"
    clean_old_packages "$pm"
  fi
  clean_user_data
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

if [[ "$install_system" == true ]]; then
  log "Installing with ${pm} (system privileges required)"

  if [[ "$pm" == "dnf" ]]; then
    run_as_root dnf install -y "$package_path"
  elif [[ "$pm" == "apt" ]]; then
    run_as_root cp "$package_path" /var/cache/apt/archives/
    run_as_root apt install -y "/var/cache/apt/archives/${BIN_NAME}.${package_ext}"
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
    if [[ ! -f "$desktop_file" ]]; then
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
    fi

    icon_file="${icon_dir}/voquill.svg"
    if [[ ! -f "$icon_file" ]]; then
      log "Embedding icon from AppImage"
      extraction_dir="${tmp_dir}/appimage-extract"
      mkdir -p "$extraction_dir"
      if "${appimage_path}" --appimage-extract >/dev/null 2>&1; then
        find "$extraction_dir" -name "*.png" -o -name "*.svg" 2>/dev/null | head -n1 | while read -r ico; do
          cp "$ico" "$icon_file" 2>/dev/null || true
        done
        rm -rf "$extraction_dir"
      fi
    fi
  fi
fi

if optional_cmd gtk-update-icon-cache; then
  if [[ "$install_system" == true ]]; then
    run_as_root gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
  else
    gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
  fi
fi

if optional_cmd update-desktop-database; then
  if [[ "$install_system" == true ]]; then
    run_as_root update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
  else
    update-desktop-database "${HOME}/.local/share/applications" >/dev/null 2>&1 || true
  fi
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