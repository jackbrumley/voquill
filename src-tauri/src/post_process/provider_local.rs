use crate::model_manager::ModelManager;
use crate::post_process::{PostProcessError, PostProcessService};
use async_trait::async_trait;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Child;
use tokio::sync::Mutex;

const BINARY_VERSION: &str = "b10331";
const PORT_START: u16 = 6101;
const PORT_END: u16 = 6200;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

pub struct SidecarPostProcess {
    port: u16,
    _process: Arc<Mutex<Option<Child>>>,
    model_size: String,
    use_gpu: bool,
}

impl SidecarPostProcess {
    /// Starts the llama-server sidecar for the given post-process engine. GPU
    /// engines try the Vulkan build first and fall back to the CPU build,
    /// recording the reason in `last_gpu_error` (the same fallback contract
    /// as whisper GPU transcription).
    pub async fn new(
        engine_name: &str,
        model_size: &str,
        last_gpu_error: Arc<std::sync::Mutex<Option<String>>>,
    ) -> Result<Self, PostProcessError> {
        if crate::engine_factory::engine_uses_gpu(engine_name) {
            match Self::start(engine_name, model_size, true).await {
                Ok(service) => {
                    *last_gpu_error.lock().unwrap() = None;
                    return Ok(service);
                }
                Err(error) => {
                    crate::log_warn!(
                        "llama-server GPU start failed ({}); falling back to CPU",
                        error
                    );
                    *last_gpu_error.lock().unwrap() = Some(error.to_string());
                }
            }
        }
        Self::start(engine_name, model_size, false).await
    }

    async fn start(
        engine_name: &str,
        model_size: &str,
        use_gpu: bool,
    ) -> Result<Self, PostProcessError> {
        let binary_path = crate::sidecar::ensure_binary(
            binary_dir(use_gpu)?,
            &binary_name(),
            download_spec(use_gpu)?,
        )
        .await
        .map_err(|e| PostProcessError::Api(e.to_string()))?;
        let port = crate::sidecar::find_free_port(PORT_START, PORT_END)
            .await
            .map_err(|e| PostProcessError::Api(e.to_string()))?;

        let mgr = ModelManager::new()
            .map_err(|e| PostProcessError::Api(format!("Failed to init model manager: {}", e)))?;

        let model = ModelManager::find_model(engine_name, model_size).ok_or_else(|| {
            PostProcessError::Api(format!(
                "Model '{}' not found in catalog for engine '{}'",
                model_size, engine_name
            ))
        })?;

        let model_path = mgr.get_model_path(&model);
        if !model_path.exists() {
            return Err(PostProcessError::Api(format!(
                "Model not downloaded: {}. Download it from Settings first.",
                model.label
            )));
        }

        crate::log_info!(
            "Starting llama-server with model: {} (gpu={})",
            model_path.display(),
            use_gpu
        );

        let args = vec![
            "-m".to_string(),
            model_path.to_string_lossy().to_string(),
            "--host".to_string(),
            "127.0.0.1".to_string(),
            "--port".to_string(),
            port.to_string(),
            "-ngl".to_string(),
            if use_gpu { "99" } else { "0" }.to_string(),
            // Cleanup must reproduce the full transcript, so the context has
            // to hold input + output for a max-length dictation (see
            // prompt::max_output_tokens).
            "-c".to_string(),
            "8192".to_string(),
        ];
        let mut child = crate::sidecar::spawn_sidecar(&binary_path, &args)
            .map_err(|e| PostProcessError::Api(format!("Failed to spawn llama-server: {}", e)))?;

        let stderr = child.stderr.take().unwrap();
        let ready = wait_for_ready(stderr, port, STARTUP_TIMEOUT).await;

        match ready {
            Ok(()) => {
                crate::log_info!("llama-server started on port {} (gpu={})", port, use_gpu);
                Ok(Self {
                    port,
                    _process: Arc::new(Mutex::new(Some(child))),
                    model_size: model_size.to_string(),
                    use_gpu,
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
    async fn post_process(
        &self,
        text: &str,
        system_prompt: &str,
        user_prompt_template: &str,
        max_output_tokens: u32,
    ) -> Result<String, PostProcessError> {
        let messages =
            super::prompt::build_post_process_messages(text, system_prompt, user_prompt_template);

        let body = serde_json::json!({
            "model": self.model_size,
            "messages": messages,
            "max_tokens": super::prompt::max_output_tokens(text, max_output_tokens),
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
            .trim();
        let cleaned = cleaned
            .lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(cleaned)
    }

    fn service_name(&self) -> &'static str {
        if self.use_gpu {
            "Post-Process (GPU)"
        } else {
            "Post-Process (Local)"
        }
    }
}

/// The llama.cpp release archive for this platform and backend. GPU engines
/// use the Vulkan builds (single archive, no CUDA runtime companion needed).
/// Provider-specific release knowledge; download/extract mechanics live in
/// `crate::sidecar`.
fn download_spec(use_gpu: bool) -> Result<crate::sidecar::SidecarDownload, PostProcessError> {
    let archive_name = archive_name(use_gpu)?;
    Ok(crate::sidecar::SidecarDownload {
        log_label: "llama-server",
        archive_url: format!(
            "https://github.com/ggml-org/llama.cpp/releases/download/{}/{}",
            BINARY_VERSION, archive_name
        ),
        archive_name: archive_name.to_string(),
        layout: crate::archive::ExtractLayout::Flat,
    })
}

fn binary_dir(use_gpu: bool) -> Result<PathBuf, PostProcessError> {
    // CPU and Vulkan builds extract into separate variant directories so they
    // never overwrite each other.
    let variant = if use_gpu { "vulkan" } else { "cpu" };
    let bin_dir = crate::paths::models_dir()
        .map_err(PostProcessError::Api)?
        .join("post-process")
        .join("bin")
        .join(variant);
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| PostProcessError::Api(format!("Failed to create bin dir: {}", e)))?;
    Ok(bin_dir)
}

fn binary_name() -> String {
    if cfg!(target_os = "windows") {
        "llama-server.exe".to_string()
    } else {
        "llama-server".to_string()
    }
}

fn archive_name(use_gpu: bool) -> Result<&'static str, PostProcessError> {
    match (std::env::consts::OS, std::env::consts::ARCH, use_gpu) {
        ("linux", "x86_64", false) => Ok("llama-b10331-bin-ubuntu-x64.tar.gz"),
        ("linux", "x86_64", true) => Ok("llama-b10331-bin-ubuntu-vulkan-x64.tar.gz"),
        ("windows", "x86_64", false) => Ok("llama-b10331-bin-win-cpu-x64.zip"),
        ("windows", "x86_64", true) => Ok("llama-b10331-bin-win-vulkan-x64.zip"),
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
