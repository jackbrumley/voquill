#!/usr/bin/env bash
set -euo pipefail

REPO="jackbrumley/voquill"
BIN_NAME="voquill"

usage() {
  cat <<'EOF'
Voquill bootstrap installer

Usage:
  install.sh [--system] [--version <tag>] [--channel latest|stable] [--yes] [--insecure-skip-verify]

Options:
  --system                Install system-wide via package manager (requires sudo)
  --clean                 Remove old packages and purge cached data (models, python-runner, debug)
                          before installing. Keeps config.json and history.db.
  --version <tag>         Install specific release tag (e.g. v1.5.0)
  --channel <name>        Release channel (default: latest)
  --yes                   Skip interactive confirmation prompts
  --insecure-skip-verify  Skip checksum verification (not recommended)
  -h, --help              Show this help message

Environment overrides:
  VOQUILL_INSTALL_URL     Full package URL override
  VOQUILL_CHECKSUM_URL    Full checksum URL override

Examples:
  curl -sf https://voquill.org/install.sh | bash
  curl -sf https://voquill.org/install.sh | bash -s -- --system --yes
  curl -sf https://voquill.org/install.sh | sudo bash -s -- --system --yes --clean
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

resolve_package_install_cmd() {
  local pm="$1"
  local path="$2"
  case "$pm" in
    dnf) printf "dnf install -y %s" "$path" ;;
    apt) printf "apt install -y %s" "$path" ;;
    *) fail "no package manager support for: $pm" ;;
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
  for pkg in "voquill" "org.voquill.app" "org.voquill.foss"; do
    if [[ "$pm" == "dnf" ]]; then
      # Tauri converts dots to dashes in RPM package names, try both
      for rpm_name in "$pkg" "${pkg//./-}"; do
        if rpm -q "$rpm_name" >/dev/null 2>&1; then
          log "Removing old package: ${rpm_name}"
          sudo dnf remove -y "$rpm_name" 2>/dev/null || true
          removed=true
          break
        fi
      done
    elif [[ "$pm" == "apt" ]] && dpkg -l "$pkg" >/dev/null 2>&1; then
      log "Removing old package: ${pkg}"
      sudo apt remove -y "$pkg" 2>/dev/null || true
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

if [[ "$install_system" == true ]]; then
  pm="$(detect_package_manager)"
  if [[ "$pm" == "none" ]]; then
    log "No supported package manager found (dnf/apt). Falling back to AppImage."
    install_system=false
  fi
fi

if [[ -n "${VOQUILL_INSTALL_URL:-}" ]]; then
  asset_url="$VOQUILL_INSTALL_URL"
  package_ext="${asset_url##*.}"
  if [[ -n "${VOQUILL_CHECKSUM_URL:-}" ]]; then
    checksum_url="$VOQUILL_CHECKSUM_URL"
  fi
else
  if [[ "$version" == "latest" ]]; then
    release_tag="$(fetch_latest_release_tag)"
  else
    release_tag="$version"
  fi

  if [[ "$install_system" == true ]]; then
    package_ext="$(resolve_package_ext "$pm")"
    gh_asset_name="${BIN_NAME}-${release_tag#v}-${os}-${arch}.${package_ext}"
    log "Resolving asset: ${gh_asset_name}"
    asset_url="https://github.com/${REPO}/releases/download/${release_tag}/${gh_asset_name}"
    checksum_url="${asset_url}.sha256"
  else
    gh_asset_name="${BIN_NAME}-${release_tag#v}-${os}-${arch}.AppImage"
    log "Resolving asset: ${gh_asset_name}"
    asset_url="https://github.com/${REPO}/releases/download/${release_tag}/${gh_asset_name}"
    checksum_url="${asset_url}.sha256"
  fi
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
    need_cmd sudo
    sudo -v || fail "sudo permission required for --clean with --system"
    clean_old_packages "$pm"
  fi
  clean_user_data
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

package_path="${tmp_dir}/${BIN_NAME}.${package_ext:-AppImage}"
checksum_path="${tmp_dir}/${BIN_NAME}.sha256"

log "Downloading release artifact"
log "  ${asset_url}"
if ! curl -fL --retry 3 --connect-timeout 15 -o "$package_path" "$asset_url"; then
  fail "download failed. No release artifact was found for this target yet.\nTry local development run:\n  git clone https://github.com/${REPO}\n  cd voquill\n  npm install\n  npm run tauri dev"
fi

if [[ "$skip_verify" == false ]] && [[ -n "${checksum_url:-}" ]]; then
  need_cmd sha256sum
  log "Downloading checksum"
  curl -fL --retry 3 --connect-timeout 15 -o "$checksum_path" "$checksum_url" || log "WARNING: checksum download failed, skipping verification"
  if [[ -s "$checksum_path" ]]; then
    expected="$(awk '{print $1}' "$checksum_path" | head -n1)"
    actual="$(sha256sum "$package_path" | awk '{print $1}')"
    if [[ -z "$expected" ]]; then
      log "WARNING: checksum file was empty, skipping verification"
    elif [[ "$expected" != "$actual" ]]; then
      fail "checksum mismatch"
    else
      log "Checksum verified"
    fi
  fi
elif [[ "$skip_verify" == true ]]; then
  log "WARNING: checksum verification disabled"
fi

if [[ "$install_system" == true ]]; then
  need_cmd sudo
  log "Installing with ${pm} (sudo required)"

  if [[ "$pm" == "dnf" ]]; then
    sudo dnf install -y "$package_path"
  elif [[ "$pm" == "apt" ]]; then
    sudo apt install -y "$package_path"
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
    sudo gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
  else
    gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
  fi
fi

if optional_cmd update-desktop-database; then
  if [[ "$install_system" == true ]]; then
    sudo update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
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