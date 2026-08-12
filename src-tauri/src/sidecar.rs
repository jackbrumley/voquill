//! Shared lifecycle for external sidecar server binaries (llama-server,
//! sherpa-onnx).
//!
//! Single owner for acquiring and spawning the third-party binaries that back
//! local engines: downloading release archives, extracting them via
//! `crate::archive`, finding ports, and spawning child processes with
//! consistent stdio/window behavior. Readiness probing stays with each
//! provider since it is protocol-specific.

use anyhow::{bail, Context};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};

/// Describes a downloadable sidecar release archive. Callers own their
/// provider-specific platform-to-archive mapping; this module owns the
/// download/extract mechanics.
pub struct SidecarDownload {
    /// Human-readable name used in logs, e.g. "llama-server".
    pub log_label: &'static str,
    pub archive_url: String,
    pub archive_name: String,
    pub layout: crate::archive::ExtractLayout,
}

/// Returns `bin_dir/binary_name`, downloading and extracting the release
/// archive first when the binary is not present.
pub async fn ensure_binary(
    bin_dir: PathBuf,
    binary_name: &str,
    download: SidecarDownload,
) -> anyhow::Result<PathBuf> {
    let binary_path = bin_dir.join(binary_name);
    if binary_path.exists() {
        return Ok(binary_path);
    }

    crate::log_info!("{} binary not found, downloading...", download.log_label);
    download_and_extract(&bin_dir, &download).await?;

    if !binary_path.exists() {
        bail!(
            "{} downloaded but {} not found in archive",
            download.log_label,
            binary_name
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&binary_path) {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o111);
            let _ = std::fs::set_permissions(&binary_path, perms);
        }
    }

    Ok(binary_path)
}

async fn download_and_extract(
    target_dir: &std::path::Path,
    download: &SidecarDownload,
) -> anyhow::Result<()> {
    crate::log_info!(
        "Downloading {} from {}",
        download.log_label,
        download.archive_url
    );

    let response = reqwest::get(&download.archive_url)
        .await
        .with_context(|| format!("{} download failed", download.log_label))?
        .error_for_status()
        .with_context(|| format!("{} download returned HTTP error", download.log_label))?;

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_logged_percent: i32 = -1;

    let archive_path = target_dir.join(&download.archive_name);
    {
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        let mut file = tokio::fs::File::create(&archive_path)
            .await
            .with_context(|| format!("Failed to create {}", archive_path.display()))?;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Download stream failed")?;
            file.write_all(&chunk)
                .await
                .context("Failed to write archive chunk")?;
            downloaded += chunk.len() as u64;
            if total > 0 {
                let percent = ((downloaded as f64 / total as f64) * 100.0) as i32;
                if percent != last_logged_percent {
                    crate::log_info!("{} download: {}%", download.log_label, percent);
                    last_logged_percent = percent;
                }
            }
        }
        file.flush().await.context("Failed to flush archive")?;
    }

    crate::log_info!("Extracting {}...", download.archive_name);
    crate::archive::extract_archive(&archive_path, target_dir, download.layout)
        .with_context(|| format!("Failed to extract {}", download.archive_name))?;

    let _ = std::fs::remove_file(&archive_path);
    crate::log_info!("{} binary downloaded and extracted", download.log_label);
    Ok(())
}

/// Returns the first TCP port in `[start, end]` with nothing listening on
/// localhost.
pub async fn find_free_port(start: u16, end: u16) -> anyhow::Result<u16> {
    for port in start..=end {
        if TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .is_err()
        {
            return Ok(port);
        }
    }
    bail!("No free ports available ({}-{})", start, end)
}

/// Spawns a sidecar server process with consistent behavior: stdout/stdin
/// silenced, stderr piped for readiness probing, killed when the handle is
/// dropped, and no console window popping up on Windows.
pub fn spawn_sidecar(binary_path: &std::path::Path, args: &[String]) -> anyhow::Result<Child> {
    let mut command = Command::new(binary_path);
    command
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true);

    #[cfg(target_os = "windows")]
    {
        // CREATE_NO_WINDOW: prevent the console-subsystem sidecar from
        // popping up (and stealing keyboard focus on) Windows.
        command.creation_flags(0x08000000);
    }

    command
        .spawn()
        .with_context(|| format!("Failed to spawn {}", binary_path.display()))
}
