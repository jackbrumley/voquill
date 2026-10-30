#!/usr/bin/env bash
# Voquill installer for macOS (Apple Silicon). install.sh hands off to this
# script on macOS, so the one-line install command is the same everywhere.
#
# Must stay compatible with the bash 3.2 that ships with macOS, and must not
# need python3 (absent on a fresh Mac): JSON is read with JavaScript for
# Automation (osascript), which every macOS install includes.
#
# Downloads made with curl carry no quarantine attribute, so Gatekeeper does
# not block the ad-hoc signed (not notarized) app bundle.
set -euo pipefail

REPO="jackbrumley/voquill"
BIN_NAME="voquill"
APP_NAME="Voquill.app"
INSTALL_DIR="/Applications"

# GitHub API request headers. VOQUILL_GITHUB_TOKEN is optional: anonymous
# requests are limited to 60 an hour per IP address, which shared networks
# (and CI runners) can exhaust.
GITHUB_API_HEADERS=(-H "Accept: application/vnd.github+json")
if [[ -n "${VOQUILL_GITHUB_TOKEN:-}" ]]; then
  GITHUB_API_HEADERS+=(-H "Authorization: Bearer ${VOQUILL_GITHUB_TOKEN}")
fi

usage() {
  cat <<'EOF'
Voquill installer for macOS (Apple Silicon)

Usage:
  install.sh [--version <tag>] [--clean] [--yes] [--no-relaunch] [--insecure-skip-verify]

Installs Voquill.app into /Applications.

Options:
  --clean                 Remove cached data (models, python-runner, debug) before installing.
                          Keeps config.json and history.db.
  --version <tag>         Install specific release tag (e.g. v1.8.0)
  --channel <name>        Release channel (default: latest)
  --yes                   Skip interactive confirmation prompts
  --no-relaunch           Do not launch Voquill after installation (launches by default)
  --relaunch              Explicitly launch Voquill after installation (default behavior)
  --insecure-skip-verify  Skip checksum verification (not recommended)
  -h, --help              Show this help message

Environment overrides:
  VOQUILL_NO_RELAUNCH     Set to 1 to disable launching Voquill after install

Example:
  curl -sf https://voquill.org/install.sh | bash
EOF
}

LOG_DIR="${HOME}/.config/voquill-app/debug"
mkdir -p "$LOG_DIR" 2>/dev/null || true
LOG_FILE="${LOG_DIR}/update.log"

log() {
  printf "[voquill] %s\n" "$*"
  printf "[%s] [voquill] %s\n" "$(date '+%Y-%m-%d %H:%M:%S')" "$*" >> "$LOG_FILE" 2>/dev/null || true
}

fail() {
  printf "[voquill] ERROR: %s\n" "$*" >&2
  printf "[%s] [voquill] ERROR: %s\n" "$(date '+%Y-%m-%d %H:%M:%S')" "$*" >> "$LOG_FILE" 2>/dev/null || true
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

# Reports the hardware, not the shell: under Rosetta `uname -m` says x86_64
# even on Apple Silicon.
require_apple_silicon() {
  if [[ "$(sysctl -n hw.optional.arm64 2>/dev/null || printf 0)" != "1" ]]; then
    fail "Voquill for macOS requires an Apple Silicon Mac (M1 or newer); Intel Macs are not supported."
  fi
}

# Prints one field from a GitHub release JSON file. `asset_digest <name>`
# selects the digest of the named asset; `tag_name` selects the release tag.
read_release_field() {
  local json_file="$1"
  local field="$2"
  local asset_name="${3:-}"
  local reader_script="${tmp_dir}/read-release-field.js"
  if [[ ! -f "$reader_script" ]]; then
    write_release_field_reader "$reader_script"
  fi
  osascript -l JavaScript "$reader_script" "$json_file" "$field" "$asset_name"
}

write_release_field_reader() {
  cat > "$1" <<'EOF'
ObjC.import("Foundation");
function run(argv) {
  const text = $.NSString.stringWithContentsOfFileEncodingError(argv[0], $.NSUTF8StringEncoding, null);
  const release = JSON.parse(ObjC.unwrap(text));
  if (argv[1] === "tag_name") {
    return release.tag_name || "";
  }
  const asset = (release.assets || []).find((candidate) => candidate.name === argv[2]);
  return asset && asset.digest ? asset.digest : "";
}
EOF
}

fetch_release_json() {
  local release_path="$1"
  local output_file="$2"
  curl -fsSL --retry 3 --connect-timeout 10 "${GITHUB_API_HEADERS[@]}" \
    -o "$output_file" "https://api.github.com/repos/${REPO}/releases/${release_path}" \
    || fail "could not fetch release information from the GitHub API"
}

stop_running_app() {
  if pgrep -x "$BIN_NAME" >/dev/null 2>&1; then
    log "Stopping running Voquill"
    osascript -e 'tell application "Voquill" to quit' >/dev/null 2>&1 || true
    sleep 2
    pkill -KILL -x "$BIN_NAME" >/dev/null 2>&1 || true
  fi
}

clean_user_data() {
  local config_dir="${HOME}/.config/voquill-app"
  if [[ ! -d "$config_dir" ]]; then
    log "No user data directory found, skipping cleanup"
    return
  fi
  log "Cleaning cached user data (keeping config.json and history.db)"
  local dir
  for dir in models python-runner debug; do
    if [[ -d "${config_dir}/${dir}" ]]; then
      log "  Removing ${dir}/"
      rm -rf "${config_dir:?}/${dir}"
    fi
  done
}

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
    --appimage|--system)
      fail "$1 is a Linux install mode; on macOS Voquill installs into ${INSTALL_DIR}."
      ;;
    --clean) clean=true ;;
    --relaunch) relaunch=true ;;
    --no-relaunch) relaunch=false ;;
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
    --yes) non_interactive=true ;;
    --insecure-skip-verify) skip_verify=true ;;
    -h|--help)
      usage
      exit 0
      ;;
    *) fail "unknown option: $1" ;;
  esac
  shift
done

if [[ "$channel" != "latest" && "$channel" != "stable" ]]; then
  fail "unsupported channel: $channel"
fi

need_cmd curl
need_cmd ditto
need_cmd osascript
require_apple_silicon

if [[ ! -t 0 ]]; then
  non_interactive=true
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
release_json="${tmp_dir}/release.json"

if [[ -z "$version" ]]; then
  fetch_release_json "latest" "$release_json"
else
  fetch_release_json "tags/${version}" "$release_json"
fi
release_tag="$(read_release_field "$release_json" tag_name)"
[[ -n "$release_tag" ]] || fail "could not determine the release tag from the GitHub API"

asset_name="${BIN_NAME}-${release_tag#v}-macos-arm64.zip"
asset_url="https://github.com/${REPO}/releases/download/${release_tag}/${asset_name}"

log "Preparing Voquill install"
log "Release: ${release_tag}"
log "Install location: ${INSTALL_DIR}/${APP_NAME}"

if [[ "$non_interactive" == false ]]; then
  printf "Proceed with installation? [y/N] "
  read -r answer
  if [[ "${answer:-}" != "y" && "${answer:-}" != "Y" ]]; then
    fail "installation cancelled"
  fi
else
  log "Non-interactive mode detected, continuing automatically"
fi

package_path="${tmp_dir}/${asset_name}"
log "Downloading release artifact"
log "  ${asset_url}"
curl -fL --retry 3 --connect-timeout 15 -o "$package_path" "$asset_url" \
  || fail "download failed: ${asset_name} is not available for ${release_tag}"

# GitHub computes a SHA-256 digest for every uploaded release asset; that
# digest is the single source of truth for verification.
if [[ "$skip_verify" == true ]]; then
  log "WARNING: checksum verification disabled"
else
  expected_digest="$(read_release_field "$release_json" asset_digest "$asset_name")"
  [[ -n "$expected_digest" ]] || fail "could not find the checksum for ${asset_name}. Re-run later, or with --insecure-skip-verify to bypass verification."
  actual_digest="$(shasum -a 256 "$package_path" | awk '{print $1}')"
  if [[ "${expected_digest#sha256:}" != "$actual_digest" ]]; then
    fail "checksum mismatch (expected ${expected_digest#sha256:}, got ${actual_digest})"
  fi
  log "Checksum verified"
fi

log "Extracting ${APP_NAME}"
ditto -x -k "$package_path" "$tmp_dir/extracted"
[[ -d "${tmp_dir}/extracted/${APP_NAME}" ]] || fail "the downloaded archive does not contain ${APP_NAME}"

stop_running_app

if [[ "$clean" == true ]]; then
  log "Clean mode enabled"
  clean_user_data
fi

# /Applications is writable by administrator accounts without sudo; standard
# accounts are asked for an administrator password.
install_command=()
if [[ ! -w "$INSTALL_DIR" ]]; then
  log "Administrator permission required to write to ${INSTALL_DIR}"
  install_command=(sudo)
fi

log "Installing ${APP_NAME} into ${INSTALL_DIR}"
"${install_command[@]+"${install_command[@]}"}" rm -rf "${INSTALL_DIR:?}/${APP_NAME}"
"${install_command[@]+"${install_command[@]}"}" ditto "${tmp_dir}/extracted/${APP_NAME}" "${INSTALL_DIR}/${APP_NAME}"

log "Installation complete"
log "On first launch, grant Microphone, Accessibility and (for Fn hotkeys) Input Monitoring permission in System Settings → Privacy & Security."

if [[ "$relaunch" == true ]]; then
  log "Launching Voquill..."
  open "${INSTALL_DIR}/${APP_NAME}" || log "Could not launch Voquill automatically; open it from Applications."
fi
