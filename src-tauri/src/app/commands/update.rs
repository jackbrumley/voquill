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
        "📡 Tauri Command: check_for_updates invoked (current={})",
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
            "🧭 Update available: {} -> {} ({})",
            current_version,
            latest_version,
            latest_release.html_url
        );
    } else {
        crate::log_info!(
            "🧭 No update available (current={}, latest={})",
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
