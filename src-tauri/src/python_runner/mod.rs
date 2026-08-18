use crate::diarization::DiarizationResult;
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tauri::Manager;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
const RUNNER_PORT_START: u16 = 6201;
const RUNNER_PORT_END: u16 = 6350;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
const HEALTH_RETRY_INTERVAL: Duration = Duration::from_millis(500);
const RUNNER_VERSION: &str = "1.1.0";
const PYTHON_VERSION: &str = "20250115";
const PYTHON_DOWNLOAD_BASE: &str =
    "https://github.com/astral-sh/python-build-standalone/releases/download";

#[derive(Clone)]
pub struct PythonRunner {
    #[allow(dead_code)]
    process: Arc<Mutex<Option<Child>>>,
    base_url: String,
    #[allow(dead_code)]
    runner_dir: PathBuf,
}

impl PythonRunner {
    pub async fn start(app_handle: &tauri::AppHandle) -> Result<Self, String> {
        let runner_dir = crate::paths::python_runner_dir()?;

        ensure_extracted(app_handle, &runner_dir).await?;
        ensure_portable_python(&runner_dir).await?;
        ensure_venv(&runner_dir).await?;
        ensure_deps(&runner_dir).await?;

        let port = find_free_port().await?;
        let mut process = spawn_server(&runner_dir, port).await?;
        let base_url = format!("http://127.0.0.1:{}", port);

        wait_for_health(&mut process, &base_url).await?;

        crate::log_info!(
            "Python runner started on port {} (dir={})",
            port,
            runner_dir.display()
        );
        Ok(Self {
            process: Arc::new(Mutex::new(Some(process))),
            base_url,
            runner_dir,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn diarize(
        &self,
        audio_path: &str,
        cluster_threshold: f32,
    ) -> Result<DiarizationResult, String> {
        client::diarize(&self.base_url, audio_path, cluster_threshold).await
    }

    pub async fn enhance(
        &self,
        audio_path: &str,
        noise_reduction_strength: f32,
    ) -> Result<String, String> {
        client::enhance(&self.base_url, audio_path, noise_reduction_strength).await
    }
}

// ── Command helper ────────────────────────────────────────────────────────

fn silent_command(program: impl AsRef<Path>) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program.as_ref());
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000);
    }
    cmd
}

// ── Portable Python download ──────────────────────────────────────────────

fn portable_python_archive_name() -> Result<String, String> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    let triple = match (os, arch) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        _ => return Err(format!("Unsupported platform: {}-{}", os, arch)),
    };
    Ok(format!(
        "cpython-3.12.8+{}-{}-install_only.tar.gz",
        PYTHON_VERSION, triple
    ))
}

fn portable_python_url() -> Result<String, String> {
    let name = portable_python_archive_name()?;
    Ok(format!(
        "{}/{}/{}",
        PYTHON_DOWNLOAD_BASE, PYTHON_VERSION, name
    ))
}

/// Download and extract a portable (relocatable) Python build if not present.
async fn ensure_portable_python(runner_dir: &Path) -> Result<(), String> {
    let python_dir = runner_dir.join("python");
    let python_bin = if cfg!(target_os = "windows") {
        python_dir.join("python.exe")
    } else {
        python_dir.join("bin").join("python3")
    };

    if !python_bin.exists() {
        let url = portable_python_url()?;
        let archive_name = portable_python_archive_name()?;
        let archive_path = runner_dir.join(&archive_name);

        crate::log_info!("Downloading portable Python from {}", url);

        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("Failed to download portable Python: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Portable Python download returned HTTP error: {}", e))?;

        let total = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut last_percent: i32 = -1;

        {
            let mut file = tokio::fs::File::create(&archive_path)
                .await
                .map_err(|e| format!("Failed to create archive file: {}", e))?;

            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
                file.write_all(&chunk)
                    .await
                    .map_err(|e| format!("Failed to write archive: {}", e))?;
                downloaded += chunk.len() as u64;
                if total > 0 {
                    let percent = ((downloaded as f64 / total as f64) * 100.0) as i32;
                    if percent != last_percent {
                        crate::log_info!("Python download: {}%", percent);
                        last_percent = percent;
                    }
                }
            }
            file.flush()
                .await
                .map_err(|e| format!("Failed to flush archive: {}", e))?;
        }

        crate::log_info!("Extracting portable Python...");
        crate::archive::extract_archive(
            &archive_path,
            &python_dir,
            crate::archive::ExtractLayout::PreservePaths,
        )
        .map_err(|e| format!("Failed to extract portable Python: {}", e))?;

        let _ = std::fs::remove_file(&archive_path);

        if !python_bin.exists() {
            return Err(format!(
                "Portable Python extracted but binary not found at {}",
                python_bin.display()
            ));
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&python_bin) {
            let perms = meta.permissions();
            if perms.mode() & 0o111 == 0 {
                let mut new_perms = perms;
                new_perms.set_mode(new_perms.mode() | 0o111);
                let _ = std::fs::set_permissions(&python_bin, new_perms);
            }
        }
    }

    crate::log_info!("Portable Python ready at {}", python_bin.display());
    Ok(())
}

fn portable_python_bin(runner_dir: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        runner_dir.join("python").join("python.exe")
    } else {
        runner_dir.join("python").join("bin").join("python3")
    }
}

/// Create a venv at runner_dir/venv using the portable Python.
async fn ensure_venv(runner_dir: &Path) -> Result<(), String> {
    let venv_python = python_bin_path(runner_dir);
    if venv_python.exists() {
        return Ok(());
    }

    create_venv(runner_dir).await
}

async fn create_venv(runner_dir: &Path) -> Result<(), String> {
    let venv_dir = runner_dir.join("venv");
    let portable = portable_python_bin(runner_dir);
    crate::log_info!(
        "Creating Python venv at {} using {}",
        venv_dir.display(),
        portable.display()
    );

    if venv_dir.exists() {
        let _ = std::fs::remove_dir_all(&venv_dir);
    }

    let output = silent_command(&portable)
        .args(["-m", "venv", &venv_dir.to_string_lossy()])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run portable Python venv: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        crate::log_warn!(
            "First venv creation attempt failed ({}); retrying after cleanup",
            stderr.trim()
        );
        let _ = std::fs::remove_dir_all(&venv_dir);
        let output = silent_command(&portable)
            .args(["-m", "venv", &venv_dir.to_string_lossy()])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to run portable Python venv (retry): {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to create venv: {}", stderr));
        }
    }

    crate::log_info!("Python venv created");
    Ok(())
}

// ── Module-level helpers ──────────────────────────────────────────────────

async fn ensure_extracted(app_handle: &tauri::AppHandle, runner_dir: &Path) -> Result<(), String> {
    let version_path = runner_dir.join(".version");

    let needs_extract = if !runner_dir.exists() || !version_path.exists() {
        true
    } else {
        match std::fs::read_to_string(&version_path) {
            Ok(v) => v.trim() != RUNNER_VERSION,
            Err(_) => true,
        }
    };

    if !needs_extract {
        return Ok(());
    }

    crate::log_info!(
        "Extracting python-runner v{} to {}",
        RUNNER_VERSION,
        runner_dir.display()
    );

    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;
    let mut src_dir = resource_dir.join("python-runner");

    if !src_dir.exists() {
        // Dev mode fallback: look at the project root (parent of src-tauri/)
        let cargo_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let project_root = cargo_dir.parent().unwrap();
        src_dir = project_root.join("python-runner");
        if !src_dir.exists() {
            return Err(format!(
                "python-runner source not found at {} or {} (dev: run from project root, build: bundled as resource)",
                resource_dir.join("python-runner").display(),
                src_dir.display()
            ));
        }
    }

    // Copy source files and modules, preserving runtime data (python/, venv/, models/)
    copy_dir_recursive(&src_dir, runner_dir)
        .map_err(|e| format!("Failed to copy python-runner: {}", e))?;

    std::fs::write(&version_path, RUNNER_VERSION)
        .map_err(|e| format!("Failed to write .version: {}", e))?;

    crate::log_info!("python-runner extracted successfully");
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

async fn find_free_port() -> Result<u16, String> {
    for port in RUNNER_PORT_START..=RUNNER_PORT_END {
        if TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .is_err()
        {
            return Ok(port);
        }
    }
    Err(format!(
        "No free ports in range {}-{}",
        RUNNER_PORT_START, RUNNER_PORT_END
    ))
}

fn collect_requirements(runner_dir: &Path) -> Result<String, String> {
    let req_dir = runner_dir.join("requirements");
    let mut all_reqs = String::new();
    if req_dir.exists() {
        let mut entries: Vec<_> = std::fs::read_dir(&req_dir)
            .map_err(|e| format!("Failed to read requirements dir: {}", e))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        entries.sort();

        for path in &entries {
            if let Ok(content) = std::fs::read_to_string(path) {
                all_reqs.push_str(&content);
                all_reqs.push('\n');
            }
        }
    }
    Ok(all_reqs)
}

async fn check_deps(python_bin: &Path) -> bool {
    silent_command(python_bin)
        .args([
            "-c",
            "import fastapi, uvicorn, pydantic, sherpa_onnx, soundfile, noisereduce, numpy, scipy",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn run_pip_install(runner_dir: &Path, all_reqs: &str) -> Result<(), String> {
    let python_bin = python_bin_path(runner_dir);
    let req_file = runner_dir.join(".combined-requirements.txt");
    std::fs::write(&req_file, all_reqs)
        .map_err(|e| format!("Failed to write combined requirements: {}", e))?;

    let output = silent_command(&python_bin)
        .args(["-m", "pip", "install", "-r", &req_file.to_string_lossy()])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to run pip install: {}", e))?;

    let _ = std::fs::remove_file(&req_file);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pip install failed: {}", stderr));
    }
    Ok(())
}

async fn ensure_deps(runner_dir: &Path) -> Result<(), String> {
    let python_bin = python_bin_path(runner_dir);
    if check_deps(&python_bin).await {
        return Ok(());
    }

    let all_reqs = collect_requirements(runner_dir)?;
    if all_reqs.trim().is_empty() {
        return Ok(());
    }

    crate::log_info!("Installing Python dependencies...");
    if run_pip_install(runner_dir, &all_reqs).await.is_ok() && check_deps(&python_bin).await {
        crate::log_info!("Python dependencies installed and verified");
        return Ok(());
    }

    // Recreate clean venv if packages or metadata were corrupted
    crate::log_warn!("Dependency check failed — recreating clean venv...");
    create_venv(runner_dir).await?;
    run_pip_install(runner_dir, &all_reqs).await?;

    if !check_deps(&python_bin).await {
        return Err("Python dependencies failed verification after fresh install".into());
    }

    crate::log_info!("Python dependencies installed and verified");
    Ok(())
}

fn python_bin_path(runner_dir: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        runner_dir.join("venv").join("Scripts").join("python.exe")
    } else {
        runner_dir.join("venv").join("bin").join("python")
    }
}

async fn spawn_server(runner_dir: &Path, port: u16) -> Result<Child, String> {
    let python_bin = python_bin_path(runner_dir);
    let server_script = runner_dir.join("server.py");

    crate::log_info!("Starting Python runner on port {}", port);

    let server_script_str = server_script
        .to_str()
        .ok_or_else(|| "Invalid server.py path".to_string())?;
    let child = silent_command(&python_bin)
        .args([server_script_str])
        .env("VOQUILL_PORT", port.to_string())
        .env(
            "VOQUILL_PYTHON_RUNNER_DIR",
            runner_dir.to_string_lossy().as_ref(),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("Failed to spawn Python runner: {}", e))?;

    Ok(child)
}

async fn wait_for_health(process: &mut Child, base_url: &str) -> Result<(), String> {
    let health_url = format!("{}/health", base_url);
    let start = std::time::Instant::now();

    while start.elapsed() < STARTUP_TIMEOUT {
        match process.try_wait() {
            Ok(Some(status)) => {
                let stderr_output = if let Some(mut stderr) = process.stderr.take() {
                    use tokio::io::AsyncReadExt;
                    let mut buf = String::new();
                    let _ = stderr.read_to_string(&mut buf).await;
                    buf
                } else {
                    String::new()
                };
                return Err(format!(
                    "Python runner exited prematurely with status {} ({})",
                    status,
                    stderr_output.trim()
                ));
            }
            Ok(None) => {}
            Err(e) => {
                crate::log_warn!("Failed to check Python runner status: {}", e);
            }
        }

        match reqwest::get(&health_url).await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            _ => tokio::time::sleep(HEALTH_RETRY_INTERVAL).await,
        }
    }

    Err(format!(
        "Python runner did not become healthy within {}s",
        STARTUP_TIMEOUT.as_secs()
    ))
}

// ── HTTP client ───────────────────────────────────────────────────────────
mod client;
