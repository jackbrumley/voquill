use crate::transcription::{TranscriptionError, TranscriptionService};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Child;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;

const BINARY_VERSION: &str = "1.13.4";
const PORT_START: u16 = 6006;
const PORT_END: u16 = 6080;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(300);

/// Manages a sherpa-onnx-ws sidecar process and provides offline transcription
/// via WebSocket.
pub struct ParakeetService {
    port: u16,
    #[allow(dead_code)]
    process: Arc<Mutex<Option<Child>>>,
    #[allow(dead_code)]
    model_size: String,
}

impl ParakeetService {
    pub async fn new(
        model_dir: PathBuf,
        model_size: &str,
        num_threads: usize,
    ) -> Result<Self, TranscriptionError> {
        let binary_path = resolve_or_download_binary().await?;
        let port = crate::sidecar::find_free_port(PORT_START, PORT_END)
            .await
            .map_err(|e| TranscriptionError::Model(e.to_string()))?;

        let args = vec![
            format!("--tokens={}", model_dir.join("tokens.txt").display()),
            format!(
                "--encoder={}",
                model_dir.join("encoder.int8.onnx").display()
            ),
            format!(
                "--decoder={}",
                model_dir.join("decoder.int8.onnx").display()
            ),
            format!("--joiner={}", model_dir.join("joiner.int8.onnx").display()),
            format!("--port={}", port),
            format!("--num-threads={}", num_threads),
            format!("--num-work-threads={}", num_threads),
            "--log-file=".to_string(),
        ];
        let mut child = crate::sidecar::spawn_sidecar(&binary_path, &args).map_err(|e| {
            TranscriptionError::Model(format!("Failed to spawn sherpa-onnx: {}", e))
        })?;

        let stderr = child.stderr.take().unwrap();
        let mut stderr_reader = tokio::io::BufReader::new(stderr);
        let ready = wait_for_ready(&mut stderr_reader, STARTUP_TIMEOUT).await;

        match ready {
            Ok(()) => {
                // Drain stderr in background so the server doesn't get SIGPIPE
                tokio::spawn(async move {
                    use tokio::io::AsyncBufReadExt;
                    let mut lines = stderr_reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        crate::log_info!("sherpa-onnx: {}", line);
                    }
                });
                crate::log_info!("Parakeet sidecar started on port {}", port);
                Ok(Self {
                    port,
                    process: Arc::new(Mutex::new(Some(child))),
                    model_size: model_size.to_string(),
                })
            }
            Err(e) => {
                let _ = child.kill().await;
                Err(TranscriptionError::Model(format!(
                    "Parakeet sidecar failed to start: {}",
                    e
                )))
            }
        }
    }
}

#[async_trait]
impl TranscriptionService for ParakeetService {
    async fn transcribe(
        &self,
        audio_data: &[u8],
        _language: Option<&str>,
        _prompt: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        let start_time = std::time::Instant::now();
        let samples = wav_to_float32(audio_data)?;

        if is_silent(&samples) {
            crate::log_info!("Parakeet: audio is silent, skipping transcription");
            return Ok(String::new());
        }

        let url = format!("ws://127.0.0.1:{}", self.port);
        let (mut ws, _) = connect_async(&url)
            .await
            .map_err(|e| TranscriptionError::Network(format!("WebSocket connect failed: {}", e)))?;

        let sample_rate: i32 = 16000;
        let num_bytes = (samples.len() * 4) as i32;
        let mut msg = Vec::with_capacity(8 + samples.len() * 4);
        msg.extend_from_slice(&sample_rate.to_le_bytes());
        msg.extend_from_slice(&num_bytes.to_le_bytes());
        for &sample in &samples {
            msg.extend_from_slice(&sample.to_le_bytes());
        }

        ws.send(tokio_tungstenite::tungstenite::Message::Binary(msg.into()))
            .await
            .map_err(|e| TranscriptionError::Network(format!("WebSocket send failed: {}", e)))?;

        let result = tokio::time::timeout(TRANSCRIBE_TIMEOUT, async {
            let mut text = String::new();
            while let Some(msg) = ws.next().await {
                let msg = msg.map_err(|e| {
                    TranscriptionError::Network(format!("WebSocket recv failed: {}", e))
                })?;
                if msg.is_close() {
                    break;
                }
                if msg.is_text() || msg.is_binary() {
                    let data = msg.into_text().unwrap_or_default();
                    text.push_str(&data);
                    let _ = ws
                        .send(tokio_tungstenite::tungstenite::Message::Text("Done".into()))
                        .await;
                }
            }
            Ok::<String, TranscriptionError>(text)
        })
        .await
        .map_err(|_| TranscriptionError::Model("Transcription timed out".into()))??;

        let trimmed = parse_offline_result(&result);
        let elapsed = start_time.elapsed();
        crate::log_info!(
            "Transcription ({}, engine=Parakeet): inference={:?}, chars={}",
            self.model_size,
            elapsed,
            trimmed.len()
        );
        Ok(trimmed)
    }

    fn service_name(&self) -> &'static str {
        "Parakeet (sherpa-onnx)"
    }
}

/// Resolves the path to the sherpa-onnx binary, downloading it if necessary.
async fn resolve_or_download_binary() -> Result<PathBuf, TranscriptionError> {
    crate::sidecar::ensure_binary(binary_dir()?, &binary_name(), download_spec())
        .await
        .map_err(|e| TranscriptionError::Model(e.to_string()))
}

/// The sherpa-onnx release archive for this platform. Provider-specific
/// release knowledge; download/extract mechanics live in `crate::sidecar`.
fn download_spec() -> crate::sidecar::SidecarDownload {
    let archive_name = archive_name();
    crate::sidecar::SidecarDownload {
        log_label: "sherpa-onnx",
        archive_url: format!(
            "https://github.com/k2-fsa/sherpa-onnx/releases/download/v{}/{}",
            BINARY_VERSION, archive_name
        ),
        archive_name: archive_name.to_string(),
        layout: crate::archive::ExtractLayout::Flat,
    }
}

fn binary_dir() -> Result<PathBuf, TranscriptionError> {
    let bin_dir = crate::paths::models_dir()
        .map_err(TranscriptionError::Model)?
        .join("transcription")
        .join("parakeet")
        .join("bin");
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| TranscriptionError::Model(format!("Failed to create bin dir: {}", e)))?;
    Ok(bin_dir)
}

fn binary_name() -> String {
    if cfg!(target_os = "windows") {
        "sherpa-onnx-offline-websocket-server.exe".to_string()
    } else {
        "sherpa-onnx-offline-websocket-server".to_string()
    }
}

fn archive_name() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "sherpa-onnx-v1.13.4-linux-x64-shared.tar.bz2",
        ("windows", "x86_64") => "sherpa-onnx-v1.13.4-win-x64-shared-MD-Release.tar.bz2",
        ("macos", _) => "sherpa-onnx-v1.13.4-osx-universal2-shared.tar.bz2",
        (os, arch) => panic!("Unsupported platform: {}-{}", os, arch),
    }
}

async fn wait_for_ready(
    reader: &mut tokio::io::BufReader<tokio::process::ChildStderr>,
    timeout: Duration,
) -> Result<(), String> {
    use tokio::io::AsyncBufReadExt;
    let mut lines = reader.lines();
    let mut captured_lines = Vec::new();

    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        tokio::time::sleep(Duration::from_millis(50)).await;
        match tokio::time::timeout(Duration::from_millis(200), lines.next_line()).await {
            Ok(Ok(Some(line))) => {
                crate::log_info!("sherpa-onnx: {}", line);
                if line.contains("Listening on:") {
                    return Ok(());
                }
                captured_lines.push(line);
            }
            Ok(Ok(None)) => {
                let error_detail = if captured_lines.is_empty() {
                    "Process exited before ready".to_string()
                } else {
                    captured_lines.join(" | ")
                };
                return Err(format!("Process exited before ready: {}", error_detail));
            }
            Ok(Err(e)) => return Err(format!("Error reading stderr: {}", e)),
            Err(_) => {}
        }
    }
    Err("Startup timeout: sidecar did not become ready within 60s".into())
}

fn wav_to_float32(wav_bytes: &[u8]) -> Result<Vec<f32>, TranscriptionError> {
    let mut reader = hound::WavReader::new(std::io::Cursor::new(wav_bytes))
        .map_err(|e| TranscriptionError::Audio(e.to_string()))?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != 16000 {
        return Err(TranscriptionError::Audio(format!(
            "Unsupported audio format: {} channels, {}Hz. Expected 1 channel, 16000Hz.",
            spec.channels, spec.sample_rate
        )));
    }
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / 32768.0))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| TranscriptionError::Audio(e.to_string()))?;
    Ok(samples)
}

fn is_silent(samples: &[f32]) -> bool {
    if samples.is_empty() {
        return true;
    }
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    rms < 0.001
}

fn parse_offline_result(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(text) = parsed.get("text").and_then(|v| v.as_str()) {
            return text.trim().to_string();
        }
    }
    trimmed.to_string()
}
