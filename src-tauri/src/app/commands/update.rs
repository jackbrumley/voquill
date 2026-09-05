use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::app::state::AppState;

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
        let state = handle.state::<AppState>();
        state.cleanup();
        handle.exit(0);
    });

    Ok(())
}

#[cfg(target_os = "linux")]
fn spawn_update_process() -> Result<(), String> {
    use std::os::unix::process::CommandExt;

    let is_appimage = std::env::var("APPIMAGE").is_ok()
        || std::env::current_exe()
            .map(|path| path.to_string_lossy().contains(".local/bin"))
            .unwrap_or(false);

    let script_cmd = if is_appimage {
        "curl -sf https://voquill.org/install.sh | bash -s -- --appimage --yes --relaunch"
    } else {
        "curl -sf https://voquill.org/install.sh | bash -s -- --system --yes --relaunch"
    };

    crate::log_info!("Spawning Linux update process: {}", script_cmd);

    let log_path = crate::paths::debug_dir()
        .map(|dir| dir.join("update.log"))
        .unwrap_or_else(|_| std::env::temp_dir().join("voquill-update.log"));

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .map_err(|error| format!("Failed to open update log file: {error}"))?;

    let log_file_err = log_file
        .try_clone()
        .map_err(|error| format!("Failed to clone update log handle: {error}"))?;

    let mut command = std::process::Command::new("bash");
    command
        .arg("-c")
        .arg(script_cmd)
        .stdin(Stdio::null())
        .stdout(log_file)
        .stderr(log_file_err);

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

    let script_cmd = "irm https://voquill.org/install.ps1 | iex -args '-Relaunch'";
    crate::log_info!("Spawning Windows update process: {}", script_cmd);

    let log_path = crate::paths::debug_dir()
        .map(|dir| dir.join("update.log"))
        .unwrap_or_else(|_| std::env::temp_dir().join("voquill-update.log"));

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .map_err(|error| format!("Failed to open update log file: {error}"))?;

    let log_file_err = log_file
        .try_clone()
        .map_err(|error| format!("Failed to clone update log handle: {error}"))?;

    let mut command = std::process::Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script_cmd,
        ])
        .stdin(Stdio::null())
        .stdout(log_file)
        .stderr(log_file_err)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);

    command
        .spawn()
        .map_err(|error| format!("Failed to spawn updater process: {error}"))?;

    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn spawn_update_process() -> Result<(), String> {
    Err("In-app updates are not supported on this platform".to_string())
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
