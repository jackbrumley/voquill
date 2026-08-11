use crate::model_manager::ModelManager;
use crate::post_process::{PostProcessError, PostProcessService};
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

const BINARY_VERSION: &str = "b10331";
const PORT_START: u16 = 6030;
const PORT_END: u16 = 6050;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub struct SidecarPostProcess {
    port: u16,
    _process: Arc<Mutex<Option<Child>>>,
    model_size: String,
    system_prompt: String,
}

impl SidecarPostProcess {
    pub async fn new(model_size: &str, system_prompt: &str) -> Result<Self, PostProcessError> {
        let binary_path = resolve_or_download_binary().await?;
        let port = find_free_port(PORT_START, PORT_END).await?;

        let mgr = ModelManager::new()
            .map_err(|e| PostProcessError::Api(format!("Failed to init model manager: {}", e)))?;

        let model =
            ModelManager::find_model("Post-Process (Local)", model_size).ok_or_else(|| {
                PostProcessError::Api(format!("Model '{}' not found in catalog", model_size))
            })?;

        let model_path = mgr.get_model_path(&model);
        if !model_path.exists() {
            return Err(PostProcessError::Api(format!(
                "Model not downloaded: {}. Download it from Settings first.",
                model.label
            )));
        }

        crate::log_info!("Starting llama-server with model: {}", model_path.display());

        let mut command = Command::new(&binary_path);
        command
            .arg("-m")
            .arg(&model_path)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("-ngl")
            .arg("0")
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

        let mut child = command
            .spawn()
            .map_err(|e| PostProcessError::Api(format!("Failed to spawn llama-server: {}", e)))?;

        let stderr = child.stderr.take().unwrap();
        let ready = wait_for_ready(stderr, port, STARTUP_TIMEOUT).await;

        match ready {
            Ok(()) => {
                crate::log_info!("llama-server started on port {}", port);
                Ok(Self {
                    port,
                    _process: Arc::new(Mutex::new(Some(child))),
                    model_size: model_size.to_string(),
                    system_prompt: system_prompt.to_string(),
                })
            }
            Err(e) => {
                let _ = child.kill().await;
                Err(PostProcessError::Api(format!(
                    "llama-server failed to start: {}",
                    e
                )))
            }
        }
    }
}

#[async_trait]
impl PostProcessService for SidecarPostProcess {
    async fn post_process(&self, text: &str) -> Result<String, PostProcessError> {
        let messages = super::prompt::build_post_process_messages(text, &self.system_prompt);

        let body = serde_json::json!({
            "model": self.model_size,
            "messages": messages,
            "max_tokens": 256,
            "temperature": 0.0,
        });

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&body)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| PostProcessError::Network(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown".to_string());
            return Err(PostProcessError::Api(format!(
                "API returned {}: {}",
                status, body_text
            )));
        }

        let data: Value = response
            .json()
            .await
            .map_err(|e| PostProcessError::Network(format!("Failed to parse response: {}", e)))?;

        let cleaned = data["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| PostProcessError::Api("No content in response".to_string()))?
            .trim()
            .to_string();
        let cleaned = cleaned.replace('\n', " ");
        let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

        Ok(cleaned)
    }

    fn service_name(&self) -> &'static str {
        "Post-Process (Local)"
    }
}

async fn resolve_or_download_binary() -> Result<PathBuf, PostProcessError> {
    let bin_dir = binary_dir()?;
    let binary_name = binary_name();
    let binary_path = bin_dir.join(&binary_name);

    if binary_path.exists() {
        return Ok(binary_path);
    }

    crate::log_info!("llama-server binary not found, downloading...");
    download_binary(&bin_dir).await?;

    if !binary_path.exists() {
        return Err(PostProcessError::Api(format!(
            "Binary downloaded but {} not found",
            binary_name
        )));
    }

    Ok(binary_path)
}

async fn download_binary(target_dir: &std::path::Path) -> Result<(), PostProcessError> {
    let archive_name = archive_name()?;
    let url = format!(
        "https://github.com/ggml-org/llama.cpp/releases/download/{}/{}",
        BINARY_VERSION, archive_name
    );

    crate::log_info!("Downloading llama-server from {}", url);

    let response = reqwest::get(&url)
        .await
        .map_err(|e| PostProcessError::Network(format!("Binary download failed: {}", e)))?;

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let archive_path = target_dir.join(archive_name);
    {
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;

        let mut file = tokio::fs::File::create(&archive_path)
            .await
            .map_err(|e| PostProcessError::Api(format!("Failed to create archive: {}", e)))?;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| PostProcessError::Network(e.to_string()))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| PostProcessError::Api(e.to_string()))?;
            downloaded += chunk.len() as u64;
            if total > 0 {
                let pct = (downloaded as f64 / total as f64) * 100.0;
                crate::log_info!("llama-server download: {:.0}%", pct);
            }
        }
        file.flush()
            .await
            .map_err(|e| PostProcessError::Api(e.to_string()))?;
    }

    crate::log_info!("Extracting {}...", archive_name);

    if archive_name.ends_with(".zip") {
        extract_zip(&archive_path, target_dir).await?;
    } else {
        extract_tar_gz(&archive_path, target_dir).await?;
    }

    let bin_name = binary_name();
    let bin_path = target_dir.join(&bin_name);
    if !bin_path.exists() {
        return Err(PostProcessError::Api(format!(
            "{} not found in archive",
            bin_name
        )));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&bin_path) {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o111);
            let _ = std::fs::set_permissions(&bin_path, perms);
        }
    }

    let _ = std::fs::remove_file(&archive_path);
    crate::log_info!("llama-server binary downloaded and extracted");
    Ok(())
}

async fn extract_tar_gz(
    archive_path: &std::path::Path,
    target_dir: &std::path::Path,
) -> Result<(), PostProcessError> {
    let mut archive = {
        let archive_file = std::fs::File::open(archive_path)
            .map_err(|e| PostProcessError::Api(format!("Failed to open archive: {}", e)))?;
        let decoder = flate2::read::GzDecoder::new(archive_file);
        tar::Archive::new(decoder)
    };

    for entry in archive
        .entries()
        .map_err(|e| PostProcessError::Api(format!("Failed to read tar entries: {}", e)))?
    {
        let mut entry =
            entry.map_err(|e| PostProcessError::Api(format!("Failed to read tar entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|_| PostProcessError::Api("Invalid path in tar".to_string()))?;

        // Strip the top-level directory (e.g. "llama-b10331/") so files go flat into target_dir
        let components: Vec<_> = path.components().collect();
        if components.len() < 2 {
            continue;
        }
        let relative: PathBuf = components[1..].iter().collect();
        let out_path = target_dir.join(&relative);

        if entry.header().entry_type().is_symlink() {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| PostProcessError::Api(format!("Failed to create dir: {}", e)))?;
            }
            #[cfg(unix)]
            {
                let target = entry
                    .link_name()
                    .map_err(|_| PostProcessError::Api("Invalid symlink target".to_string()))?
                    .ok_or_else(|| PostProcessError::Api("Symlink with no target".to_string()))?;
                std::os::unix::fs::symlink(&target, &out_path).map_err(|e| {
                    PostProcessError::Api(format!("Failed to create symlink: {}", e))
                })?;
            }
        } else if entry.header().entry_type().is_dir() {
            let _ = std::fs::create_dir_all(&out_path);
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| PostProcessError::Api(format!("Failed to create dir: {}", e)))?;
            }
            let mut out_file = std::fs::File::create(&out_path)
                .map_err(|e| PostProcessError::Api(format!("Failed to create file: {}", e)))?;
            std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| PostProcessError::Api(format!("Failed to extract file: {}", e)))?;
        }
    }

    Ok(())
}

async fn extract_zip(
    archive_path: &std::path::Path,
    target_dir: &std::path::Path,
) -> Result<(), PostProcessError> {
    let file = std::fs::File::open(archive_path)
        .map_err(|e| PostProcessError::Api(format!("Failed to open archive: {}", e)))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| PostProcessError::Api(format!("Failed to open zip archive: {}", e)))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| PostProcessError::Api(format!("Failed to read zip entry: {}", e)))?;

        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        // Skip the root directory component (e.g. "repo/") so files go flat into target_dir
        let relative: PathBuf =
            enclosed
                .components()
                .skip(1)
                .fold(PathBuf::new(), |mut acc, component| {
                    acc.push(component.as_os_str());
                    acc
                });
        if relative.as_os_str().is_empty() {
            continue;
        }
        let out_path = target_dir.join(&relative);

        if entry.is_dir() {
            let _ = std::fs::create_dir_all(&out_path);
            continue;
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PostProcessError::Api(format!("Failed to create dir: {}", e)))?;
        }

        let mut out_file = std::fs::File::create(&out_path)
            .map_err(|e| PostProcessError::Api(format!("Failed to create file: {}", e)))?;
        std::io::copy(&mut entry, &mut out_file)
            .map_err(|e| PostProcessError::Api(format!("Failed to extract file: {}", e)))?;
    }

    Ok(())
}

fn binary_dir() -> Result<PathBuf, PostProcessError> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| PostProcessError::Api("Could not find config directory".into()))?
        .join("foss-voquill")
        .join("models")
        .join("post-process")
        .join("bin");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| PostProcessError::Api(format!("Failed to create bin dir: {}", e)))?;
    Ok(config_dir)
}

fn binary_name() -> String {
    if cfg!(target_os = "windows") {
        "llama-server.exe".to_string()
    } else {
        "llama-server".to_string()
    }
}

fn archive_name() -> Result<&'static str, PostProcessError> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("llama-b10331-bin-ubuntu-x64.tar.gz"),
        ("windows", "x86_64") => Ok("llama-b10331-bin-win-cpu-x64.zip"),
        _ => Err(PostProcessError::Api(format!(
            "Unsupported platform for local post-processing: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))),
    }
}

async fn wait_for_ready(
    stderr: tokio::process::ChildStderr,
    port: u16,
    timeout: Duration,
) -> Result<(), String> {
    use tokio::io::AsyncBufReadExt;
    let reader = tokio::io::BufReader::new(stderr);
    let mut lines = reader.lines();

    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        tokio::time::sleep(Duration::from_millis(200)).await;

        match tokio::time::timeout(Duration::from_millis(500), lines.next_line()).await {
            Ok(Ok(Some(line))) => {
                crate::log_info!("llama-server: {}", line);
                if line.contains("starting HTTP server") || line.contains("server is listening") {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    return Ok(());
                }
                if line.contains("error") || line.contains("Error") {
                    crate::log_warn!("llama-server stderr: {}", line);
                }
            }
            Ok(Ok(None)) => {
                let health = check_health(port).await;
                if health {
                    return Ok(());
                }
                return Err("Process exited before ready".into());
            }
            Ok(Err(e)) => return Err(format!("Error reading stderr: {}", e)),
            Err(_) => {
                let health = check_health(port).await;
                if health {
                    return Ok(());
                }
            }
        }
    }

    let health = check_health(port).await;
    if health {
        return Ok(());
    }
    Err(format!(
        "Startup timeout: sidecar did not become ready within {}s",
        timeout.as_secs()
    ))
}

async fn check_health(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{}/health", port);
    match reqwest::get(&url).await {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

async fn find_free_port(start: u16, end: u16) -> Result<u16, PostProcessError> {
    for port in start..=end {
        if TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .is_err()
        {
            return Ok(port);
        }
    }
    Err(PostProcessError::Api(
        "No free ports available (6030-6050)".into(),
    ))
}
