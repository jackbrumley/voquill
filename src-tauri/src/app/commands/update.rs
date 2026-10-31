use std::process::Stdio;

use serde::{Deserialize, Serialize};

const GITHUB_LATEST_RELEASE_API_URL: &str =
    "https://api.github.com/repos/jackbrumley/voquill/releases/latest";
const GITHUB_RELEASES_LATEST_URL: &str = "https://github.com/jackbrumley/voquill/releases/latest";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    current_version: String,
    latest_version: String,
    update_available: bool,
    release_url: String,
    notes_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubLatestRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

#[tauri::command]
pub async fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    crate::log_info!(
        "Tauri Command: check_for_updates invoked (current={})",
        current_version
    );

    let client = reqwest::Client::builder()
        .build()
        .map_err(|error| format!("Failed to build update client: {error}"))?;

    let response = client
        .get(GITHUB_LATEST_RELEASE_API_URL)
        .header(reqwest::header::USER_AGENT, "voquill-update-checker")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| format!("Failed to fetch latest release: {error}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Latest release request failed with status {}",
            response.status()
        ));
    }

    let latest_release: GitHubLatestRelease = response
        .json()
        .await
        .map_err(|error| format!("Failed to decode latest release response: {error}"))?;

    let latest_version = normalize_version(&latest_release.tag_name).ok_or_else(|| {
        format!(
            "Invalid latest release version tag: {}",
            latest_release.tag_name
        )
    })?;
    let parsed_current = parse_version(&current_version)
        .ok_or_else(|| format!("Invalid current app version: {current_version}"))?;
    let parsed_latest = parse_version(&latest_version)
        .ok_or_else(|| format!("Invalid latest app version: {latest_version}"))?;

    let update_available = parsed_latest > parsed_current;

    if update_available {
        crate::log_info!(
            "Update available: {} -> {} ({})",
            current_version,
            latest_version,
            latest_release.html_url
        );
    } else {
        crate::log_info!(
            "No update available (current={}, latest={})",
            current_version,
            latest_version
        );
    }

    Ok(UpdateCheckResult {
        current_version,
        latest_version,
        update_available,
        release_url: latest_release.html_url,
        notes_url: latest_release
            .body
            .map(|_| GITHUB_RELEASES_LATEST_URL.to_string()),
    })
}

#[tauri::command]
pub async fn install_update(app_handle: tauri::AppHandle) -> Result<(), String> {
    crate::log_info!("Tauri Command: install_update invoked");

    spawn_update_process()?;

    let handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        crate::log_info!("Exiting Voquill for update installation");
        handle.exit(0);
    });

    Ok(())
}

/// The install command that replaces this installation in place. AppImage
/// installs must stay AppImage installs rather than switching to a package.
#[cfg(target_os = "linux")]
fn unix_update_command() -> &'static str {
    let is_appimage = std::env::var("APPIMAGE").is_ok()
        || std::env::current_exe()
            .map(|path| path.to_string_lossy().contains(".local/bin"))
            .unwrap_or(false);

    if is_appimage {
        "curl -sf https://voquill.org/install.sh | bash -s -- --appimage"
    } else {
        "curl -sf https://voquill.org/install.sh | bash"
    }
}

/// install.sh hands off to the macOS installer, which replaces the app bundle
/// in /Applications.
#[cfg(target_os = "macos")]
fn unix_update_command() -> &'static str {
    "curl -sf https://voquill.org/install.sh | bash"
}

/// The updater's output, and what the update-failed dialog opens.
fn update_log_path() -> std::path::PathBuf {
    crate::paths::debug_dir()
        .map(|dir| dir.join("update.log"))
        .unwrap_or_else(|_| std::env::temp_dir().join("voquill-update.log"))
}

/// A fresh update log, as the updater's stdout and stderr.
fn update_log_outputs() -> Result<(Stdio, Stdio), String> {
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(update_log_path())
        .map_err(|error| format!("Failed to open update log file: {error}"))?;
    let log_file_err = log_file
        .try_clone()
        .map_err(|error| format!("Failed to clone update log handle: {error}"))?;
    Ok((Stdio::from(log_file), Stdio::from(log_file_err)))
}

/// What to relaunch, with `--update-failed`, when the installer fails: the
/// AppImage file itself (its mounted executable disappears on exit),
/// otherwise this executable.
fn relaunch_target() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "linux")]
    {
        if let Some(appimage) = std::env::var_os("APPIMAGE") {
            return Some(std::path::PathBuf::from(appimage));
        }
    }
    std::env::current_exe().ok()
}

/// Wraps `value` in single quotes for bash, which takes everything inside
/// literally (an embedded quote closes, escapes and reopens).
#[cfg(any(target_os = "linux", target_os = "macos", test))]
fn bash_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Wraps `value` in single quotes for PowerShell, which escapes an embedded
/// quote by doubling it.
#[cfg(any(target_os = "windows", test))]
fn powershell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Runs the installer, and if it fails relaunches the installed version with
/// `--update-failed` so the user is told instead of the app just vanishing.
/// `pipefail` makes a failed download fail the update rather than piping an
/// empty script into bash.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn unix_update_script() -> String {
    let relaunch = match relaunch_target() {
        Some(path) => {
            let path = bash_single_quote(&path.to_string_lossy());
            format!("if [ -x {path} ]; then {path} --update-failed >/dev/null 2>&1 & fi")
        }
        None => "echo \"[voquill] No installed version found to relaunch\"".to_string(),
    };
    format!(
        r#"set -o pipefail
{install}
status=$?
if [ "$status" -ne 0 ]; then
  echo "[voquill] Update failed (exit $status); relaunching the installed version"
  {relaunch}
fi
"#,
        install = unix_update_command(),
    )
}

/// Windows counterpart of `unix_update_script`. install.ps1 runs from a file
/// rather than through `iex`, so its `exit 1` on failure ends the script, not
/// this updater, and sets `$LASTEXITCODE`. It runs with PowerShell's default
/// error handling, exactly as the one-line install does.
#[cfg(target_os = "windows")]
fn windows_update_script() -> String {
    let relaunch = match relaunch_target() {
        Some(path) => {
            let path = powershell_single_quote(&path.to_string_lossy());
            format!(
                "if (Test-Path -LiteralPath {path}) {{ Start-Process -FilePath {path} -ArgumentList '--update-failed' }}"
            )
        }
        None => "Write-Output '[voquill] No installed version found to relaunch'".to_string(),
    };
    format!(
        r#"$installer = Join-Path $env:TEMP 'voquill-update-install.ps1'
try {{
  Invoke-RestMethod -Uri 'https://voquill.org/install.ps1' -OutFile $installer -ErrorAction Stop
  $LASTEXITCODE = 0
  & $installer
  $succeeded = $LASTEXITCODE -eq 0
}} catch {{
  Write-Output "[voquill] ERROR: $_"
  $succeeded = $false
}}
if (-not $succeeded) {{
  Write-Output '[voquill] Update failed; relaunching the installed version'
  {relaunch}
}}
"#
    )
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn spawn_update_process() -> Result<(), String> {
    use std::os::unix::process::CommandExt;

    let script = unix_update_script();
    crate::log_info!("Spawning update process:\n{}", script);
    let (stdout, stderr) = update_log_outputs()?;

    let mut command = std::process::Command::new("bash");
    command
        .arg("-c")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr);

    // SAFETY: setsid only detaches the child from this process's session, so
    // the updater survives Voquill exiting.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    command
        .spawn()
        .map_err(|error| format!("Failed to spawn updater process: {error}"))?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn spawn_update_process() -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    const DETACHED_PROCESS: u32 = 0x00000008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

    let script = windows_update_script();
    crate::log_info!("Spawning Windows update process:\n{}", script);
    let (stdout, stderr) = update_log_outputs()?;

    let mut command = std::process::Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);

    command
        .spawn()
        .map_err(|error| format!("Failed to spawn updater process: {error}"))?;

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn spawn_update_process() -> Result<(), String> {
    Err("In-app updates are not supported on this platform".to_string())
}

/// Shown when the updater relaunched this (old) version after the installer
/// failed, so a failed update is never silent.
pub fn show_update_failed_dialog(app_handle: &tauri::AppHandle) {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    let log_path = update_log_path();
    crate::log_warn!(
        "Relaunched after a failed update; details in {}",
        log_path.display()
    );
    app_handle
        .dialog()
        .message(format!(
            "Voquill could not install the update, so this version is still installed.\n\nThe installer's output is in:\n{}",
            log_path.display()
        ))
        .title("Update failed")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Open Update Log".to_string(),
            "Close".to_string(),
        ))
        .show(move |open_log| {
            if open_log {
                if let Err(error) = super::diagnostics::open_with_system(&log_path) {
                    crate::log_warn!("Failed to open the update log: {}", error);
                }
            }
        });
}

fn normalize_version(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_start_matches('v');
    let parsed = parse_version(trimmed)?;
    Some(format!("{}.{}.{}", parsed.0, parsed.1, parsed.2))
}

fn parse_version(value: &str) -> Option<(u64, u64, u64)> {
    let sanitized = value.trim();
    if sanitized.is_empty() {
        return None;
    }

    let core = sanitized
        .split_once('-')
        .map_or(sanitized, |(left, _)| left);
    let mut segments = core.split('.');

    let major = segments.next()?.parse::<u64>().ok()?;
    let minor = segments.next()?.parse::<u64>().ok()?;
    let patch = segments.next()?.parse::<u64>().ok()?;

    if segments.next().is_some() {
        return None;
    }

    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::{bash_single_quote, powershell_single_quote};

    #[test]
    fn bash_quoting_keeps_paths_literal() {
        assert_eq!(bash_single_quote("/usr/bin/voquill"), "'/usr/bin/voquill'");
        assert_eq!(
            bash_single_quote("/home/o'brien/Voquill $HOME.AppImage"),
            r"'/home/o'\''brien/Voquill $HOME.AppImage'"
        );
    }

    #[test]
    fn powershell_quoting_doubles_embedded_quotes() {
        assert_eq!(
            powershell_single_quote(r"C:\Users\o'brien\AppData\Local\Voquill\voquill.exe"),
            r"'C:\Users\o''brien\AppData\Local\Voquill\voquill.exe'"
        );
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn unix_update_script_fails_on_a_failed_download_and_relaunches() {
        let script = super::unix_update_script();
        assert!(script.starts_with("set -o pipefail\n"));
        assert!(script.contains("--update-failed"));
    }
}
