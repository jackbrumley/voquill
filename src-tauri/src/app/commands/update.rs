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

    start_updater().await?;

    crate::log_info!("Exiting Voquill for update installation");
    app_handle.exit(0);
    Ok(())
}

/// Where the update script is downloaded from. `VOQUILL_UPDATE_BASE_URL`
/// replaces it so the release workflow can run a tag's scripts before they
/// are published.
const UPDATE_BASE_URL: &str = "https://voquill.org";
const UPDATE_BASE_URL_OVERRIDE_VARIABLE: &str = "VOQUILL_UPDATE_BASE_URL";

/// The first line the update script writes (see docs/update.ps1 and
/// docs/update.sh). Seeing it in the update log proves the script was
/// downloaded and is running.
const UPDATE_STARTED_MARKER: &str = "[voquill-update] started";

/// How long the update script may take to report that it started. Covers a
/// cold PowerShell start plus downloading a small script on a slow network.
const UPDATER_START_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const UPDATER_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

/// Shown when the update script never reported that it started.
const UPDATER_NOT_STARTED_MESSAGE: &str = "The updater could not start, so nothing was changed. Download the new version from the release page instead.";

/// Starts the updater and returns once the update script reports that it is
/// running; only then may Voquill exit. The app does as little as possible:
/// it starts a shell that downloads the current update script from the
/// website and runs it, so every later fix to the update procedure is a
/// change to that script, not a new release. If the script never starts
/// (the shell could not run, the download failed), this returns an error and
/// Voquill stays open, so an update can never close the app and do nothing.
pub async fn start_updater() -> Result<(), String> {
    let base_url = parse_update_base_url(std::env::var(UPDATE_BASE_URL_OVERRIDE_VARIABLE).ok())?;
    let relaunch = relaunch_target()
        .ok_or_else(|| "Could not find this Voquill executable to relaunch".to_string())?;
    let log_path = update_log_path();
    let (stdout, stderr) = update_log_outputs(&log_path)?;

    crate::log_info!(
        "Starting updater from {} (relaunch target {})",
        base_url,
        relaunch.display()
    );
    let mut updater = spawn_updater(&base_url, &relaunch, stdout, stderr)?;
    wait_for_updater_start(&mut updater, &log_path).await
}

/// The default base URL, or the override, which must be https and loses any
/// trailing slash.
fn parse_update_base_url(override_url: Option<String>) -> Result<String, String> {
    let Some(override_url) = override_url else {
        return Ok(UPDATE_BASE_URL.to_string());
    };
    let trimmed = override_url.trim().trim_end_matches('/');
    if !trimmed.starts_with("https://") {
        return Err(format!(
            "{UPDATE_BASE_URL_OVERRIDE_VARIABLE} must be an https:// URL, got '{override_url}'"
        ));
    }
    Ok(trimmed.to_string())
}

/// Waits for the started line in the update log. An updater that exits or
/// stays silent past the timeout has failed to start: it is stopped, its
/// log is copied into the session log, and the user gets an error instead of
/// a closed app.
async fn wait_for_updater_start(
    updater: &mut std::process::Child,
    log_path: &std::path::Path,
) -> Result<(), String> {
    let deadline = std::time::Instant::now() + UPDATER_START_TIMEOUT;
    loop {
        if read_update_log(log_path).contains(UPDATE_STARTED_MARKER) {
            crate::log_info!("Updater started");
            return Ok(());
        }
        let failure = match updater.try_wait() {
            Ok(Some(status)) => Some(format!("it exited ({status})")),
            Ok(None) if std::time::Instant::now() >= deadline => {
                let _ = updater.kill();
                Some(format!(
                    "it did not report starting within {}s",
                    UPDATER_START_TIMEOUT.as_secs()
                ))
            }
            Ok(None) => None,
            Err(error) => Some(format!("its state could not be read: {error}")),
        };
        if let Some(reason) = failure {
            // The script may have written its first line just before exiting.
            if read_update_log(log_path).contains(UPDATE_STARTED_MARKER) {
                crate::log_info!("Updater started");
                return Ok(());
            }
            crate::log_warn!(
                "Updater did not start: {}. Update log:\n{}",
                reason,
                read_update_log(log_path)
            );
            return Err(UPDATER_NOT_STARTED_MESSAGE.to_string());
        }
        tokio::time::sleep(UPDATER_POLL_INTERVAL).await;
    }
}

fn read_update_log(log_path: &std::path::Path) -> String {
    std::fs::read(log_path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// The updater's output, and what the update-failed dialog opens.
fn update_log_path() -> std::path::PathBuf {
    crate::paths::debug_dir()
        .map(|dir| dir.join("update.log"))
        .unwrap_or_else(|_| std::env::temp_dir().join("voquill-update.log"))
}

/// A fresh update log, as the updater's stdout and stderr. Emptied first, then
/// opened in append mode: the installers also append their own lines to this
/// file, and a plain write handle would write over them at its own offset.
fn update_log_outputs(log_path: &std::path::Path) -> Result<(Stdio, Stdio), String> {
    std::fs::write(log_path, b"")
        .map_err(|error| format!("Failed to reset update log file: {error}"))?;
    let log_file = std::fs::OpenOptions::new()
        .append(true)
        .open(log_path)
        .map_err(|error| format!("Failed to open update log file: {error}"))?;
    let log_file_err = log_file
        .try_clone()
        .map_err(|error| format!("Failed to clone update log handle: {error}"))?;
    Ok((Stdio::from(log_file), Stdio::from(log_file_err)))
}

/// What the update script relaunches, with `--update-failed`, when the
/// installer fails: the AppImage file itself (its mounted executable
/// disappears on exit), otherwise this executable.
fn relaunch_target() -> Option<std::path::PathBuf> {
    #[cfg(target_os = "linux")]
    {
        if let Some(appimage) = std::env::var_os("APPIMAGE") {
            return Some(std::path::PathBuf::from(appimage));
        }
    }
    std::env::current_exe().ok()
}

/// Wraps `value` in single quotes for PowerShell, which escapes an embedded
/// quote by doubling it.
#[cfg(any(target_os = "windows", test))]
fn powershell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// What to change in the updater's environment so it runs with the
/// computer's own programs and libraries, not the AppImage's.
#[cfg(any(target_os = "linux", test))]
#[derive(Debug, PartialEq)]
enum EnvironmentChange {
    Remove(String),
    Set(String, String),
}

/// The AppImage runtime points search paths (`LD_LIBRARY_PATH`,
/// `XDG_DATA_DIRS`, `GTK_PATH`, ...) into its own bundle at `APPDIR`. A
/// system program started from it inherits them and loads the bundled
/// libraries: the system `curl` then fails with "symbol lookup error" in
/// libcurl, so the updater could never download anything. Every entry under
/// `APPDIR` is removed; a variable left with no entries is removed entirely
/// (an empty `LD_LIBRARY_PATH` entry would mean the current directory).
/// `APPIMAGE` (the .AppImage file itself) is outside `APPDIR` and kept: the
/// update script uses it to update the AppImage install.
#[cfg(any(target_os = "linux", test))]
fn appimage_environment_changes(
    appdir: &str,
    variables: impl Iterator<Item = (String, String)>,
) -> Vec<EnvironmentChange> {
    let appdir = appdir.trim_end_matches('/');
    let is_inside_appdir =
        |entry: &str| entry == appdir || entry.starts_with(&format!("{appdir}/"));
    variables
        .filter(|(_, value)| value.split(':').any(is_inside_appdir))
        .map(|(name, value)| {
            let kept: Vec<&str> = value
                .split(':')
                .filter(|entry| !entry.is_empty() && !is_inside_appdir(entry))
                .collect();
            if kept.is_empty() {
                EnvironmentChange::Remove(name)
            } else {
                EnvironmentChange::Set(name, kept.join(":"))
            }
        })
        .collect()
}

/// Applies `appimage_environment_changes` when running as an AppImage.
#[cfg(target_os = "linux")]
fn use_system_environment(command: &mut std::process::Command) {
    let Some(appdir) = std::env::var("APPDIR").ok().filter(|dir| !dir.is_empty()) else {
        return;
    };
    let variables = std::env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)));
    for change in appimage_environment_changes(&appdir, variables) {
        crate::log_info!("Updater environment: {:?}", change);
        match change {
            EnvironmentChange::Remove(name) => {
                command.env_remove(name);
            }
            EnvironmentChange::Set(name, value) => {
                command.env(name, value);
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn use_system_environment(_command: &mut std::process::Command) {}

/// Downloads update.sh and pipes it to bash. If the download fails, bash
/// gets an empty script and exits without the started line, which
/// `wait_for_updater_start` reports. The URL and path are passed as
/// positional arguments, so they are never parsed as shell code. `setsid`
/// detaches the updater from this process's session, so it survives Voquill
/// exiting.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn spawn_updater(
    base_url: &str,
    relaunch: &std::path::Path,
    stdout: Stdio,
    stderr: Stdio,
) -> Result<std::process::Child, String> {
    use std::os::unix::process::CommandExt;

    let mut command = std::process::Command::new("bash");
    command
        .arg("-c")
        .arg(r#"curl --proto '=https' --tlsv1.2 -fsSL --retry 3 --connect-timeout 10 "$1/update.sh" | bash -s -- --base-url "$1" --relaunch "$2""#)
        .arg("voquill-update")
        .arg(base_url)
        .arg(relaunch)
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(stderr);
    use_system_environment(&mut command);

    // SAFETY: setsid only detaches the child from this process's session.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    command
        .spawn()
        .map_err(|error| format!("Failed to start the updater: {error}"))
}

/// Downloads update.ps1 and runs it as a script block. A failed download is
/// a terminating error, so PowerShell exits without the started line, which
/// `wait_for_updater_start` reports.
///
/// The process gets its own hidden console. It must not be
/// `DETACHED_PROCESS`: Windows PowerShell started without any console exits
/// at once without running a line, which made every in-app update before
/// 1.11.3 close the app and do nothing. A console of its own also keeps the
/// updater alive after Voquill exits.
#[cfg(target_os = "windows")]
fn spawn_updater(
    base_url: &str,
    relaunch: &std::path::Path,
    stdout: Stdio,
    stderr: Stdio,
) -> Result<std::process::Child, String> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

    let script = format!(
        "[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12\n\
         $updater = Invoke-RestMethod -Uri {script_url} -ErrorAction Stop\n\
         & ([scriptblock]::Create($updater)) -BaseUrl {base_url} -Relaunch {relaunch}\n",
        script_url = powershell_single_quote(&format!("{base_url}/update.ps1")),
        base_url = powershell_single_quote(base_url),
        relaunch = powershell_single_quote(&relaunch.to_string_lossy()),
    );

    std::process::Command::new("powershell.exe")
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
        .creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .map_err(|error| format!("Failed to start the updater: {error}"))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn spawn_updater(
    _base_url: &str,
    _relaunch: &std::path::Path,
    _stdout: Stdio,
    _stderr: Stdio,
) -> Result<std::process::Child, String> {
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
    use super::{
        appimage_environment_changes, parse_update_base_url, powershell_single_quote,
        EnvironmentChange, UPDATE_STARTED_MARKER,
    };

    #[test]
    fn appimage_paths_are_removed_from_the_updater_environment() {
        let variables = [
            ("APPDIR", "/tmp/appimage_extracted_1"),
            ("APPIMAGE", "/home/me/.local/bin/voquill"),
            ("LD_LIBRARY_PATH", "/tmp/appimage_extracted_1/usr/lib:"),
            (
                "XDG_DATA_DIRS",
                "/tmp/appimage_extracted_1/usr/share:/usr/local/share:/usr/share",
            ),
            ("PATH", "/tmp/appimage_extracted_1/usr/bin:/usr/bin:/bin"),
            ("HOME", "/home/me"),
            ("OTHER", "/tmp/appimage_extracted_10/usr/lib"),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_string(), value.to_string()));

        assert_eq!(
            appimage_environment_changes("/tmp/appimage_extracted_1/", variables),
            vec![
                EnvironmentChange::Remove("APPDIR".to_string()),
                EnvironmentChange::Remove("LD_LIBRARY_PATH".to_string()),
                EnvironmentChange::Set(
                    "XDG_DATA_DIRS".to_string(),
                    "/usr/local/share:/usr/share".to_string()
                ),
                EnvironmentChange::Set("PATH".to_string(), "/usr/bin:/bin".to_string()),
            ]
        );
    }

    #[test]
    fn powershell_quoting_doubles_embedded_quotes() {
        assert_eq!(
            powershell_single_quote(r"C:\Users\o'brien\AppData\Local\Voquill\voquill.exe"),
            r"'C:\Users\o''brien\AppData\Local\Voquill\voquill.exe'"
        );
    }

    #[test]
    fn update_base_url_defaults_to_the_website() {
        assert_eq!(parse_update_base_url(None).unwrap(), "https://voquill.org");
    }

    #[test]
    fn update_base_url_override_must_be_https() {
        assert_eq!(
            parse_update_base_url(Some(
                "https://raw.githubusercontent.com/jackbrumley/voquill/v1.2.3/docs/".to_string()
            ))
            .unwrap(),
            "https://raw.githubusercontent.com/jackbrumley/voquill/v1.2.3/docs"
        );
        assert!(parse_update_base_url(Some("http://127.0.0.1:8000".to_string())).is_err());
    }

    /// The app waits for this exact line, so the scripts must print it first.
    #[test]
    fn update_scripts_print_the_started_marker_first() {
        let first_output = |script: &str, command: &str| {
            script
                .lines()
                .find(|line| line.starts_with(command))
                .map(str::to_string)
        };
        assert_eq!(
            first_output(include_str!("../../../../docs/update.ps1"), "Write-Output "),
            Some(format!("Write-Output \"{UPDATE_STARTED_MARKER}\""))
        );
        assert_eq!(
            first_output(include_str!("../../../../docs/update.sh"), "echo "),
            Some(format!("echo \"{UPDATE_STARTED_MARKER}\""))
        );
    }
}
