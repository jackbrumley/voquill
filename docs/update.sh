#!/usr/bin/env bash
# Voquill's in-app updater for Linux and macOS.
#
# Update Now (from 1.11.5 on) downloads the current copy of this script and
# runs it, so this file owns the whole update. A fix made here reaches every
# installed version on its next update, with no new release. The app only
# starts bash and waits for the started line below.
#
# Contract with the app; keep it stable, every shipped version relies on it:
#   - Arguments: --base-url <where this script was downloaded from, without a
#     trailing slash> and --relaunch <the installed binary or AppImage>.
#   - The first output line is exactly "[voquill-update] started". Until the
#     app sees it in debug/update.log, it stays open; if the line never comes,
#     it tells the user and offers the download page instead of closing.
#   - Output goes to debug/update.log (the app redirects it there).
#   - The last line is "[voquill-update] finished: updated" or
#     "[voquill-update] finished: failed". On failure the installed version
#     is relaunched with --update-failed, which shows the user a dialog.

echo "[voquill-update] started"

base_url=""
relaunch=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url) base_url="$2"; shift 2 ;;
    --relaunch) relaunch="$2"; shift 2 ;;
    *) echo "[voquill-update] Ignoring unknown argument: $1"; shift ;;
  esac
done
echo "[voquill-update] $(date -u +%Y-%m-%dT%H:%M:%SZ) from ${base_url}"

finish_failed() {
  echo "[voquill-update] finished: failed"
  if [[ -n "$relaunch" && -x "$relaunch" ]]; then
    # Voquill exits once it sees the started line. Until it has, a relaunch
    # would hand off to the exiting instance (single instance) and vanish.
    for _ in $(seq 1 30); do
      pgrep -x voquill >/dev/null 2>&1 || break
      sleep 0.5
    done
    echo "[voquill-update] Relaunching the installed version"
    "$relaunch" --update-failed >/dev/null 2>&1 &
  else
    echo "[voquill-update] No installed version found to relaunch"
  fi
  exit 1
}

if [[ -z "$base_url" ]]; then
  echo "[voquill-update] ERROR: --base-url is required"
  finish_failed
fi

# Nobody is at a terminal to answer prompts. AppImage installs stay AppImage
# installs rather than switching to a package.
install_args=(--yes)
if [[ "$(uname -s)" == "Linux" ]] && { [[ -n "${APPIMAGE:-}" ]] || [[ "$relaunch" == *"/.local/bin/"* ]]; }; then
  install_args+=(--appimage)
fi

installer="$(mktemp "${TMPDIR:-/tmp}/voquill-update-install.XXXXXX")"
if ! curl --tlsv1.2 -fsSL --retry 3 --connect-timeout 10 "${base_url}/install.sh" -o "$installer"; then
  echo "[voquill-update] ERROR: could not download ${base_url}/install.sh"
  finish_failed
fi

bash "$installer" "${install_args[@]}"
status=$?
rm -f "$installer"
if [[ "$status" -ne 0 ]]; then
  echo "[voquill-update] Installer exited with status ${status}"
  finish_failed
fi

echo "[voquill-update] finished: updated"
exit 0
