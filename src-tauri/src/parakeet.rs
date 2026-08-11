use crate::transcription::{TranscriptionError, TranscriptionService};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;

const BINARY_VERSION: &str = "1.13.4";
const PORT_START: u16 = 6006;
const PORT_END: u16 = 6029;
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
    pub async fn new(model_dir: PathBuf, model_size: &str) -> Result<Self, TranscriptionError> {
        let binary_path = resolve_or_download_binary().await?;
        let port = find_free_port(PORT_START, PORT_END).await?;

        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get().saturating_sub(1).clamp(1, 4))
            .unwrap_or(2);

        let mut command = Command::new(&binary_path);
        command
            .arg(format!(
                "--tokens={}",
                model_dir.join("tokens.txt").display()
            ))
            .arg(format!(
                "--encoder={}",
                model_dir.join("encoder.int8.onnx").display()
            ))
            .arg(format!(
                "--decoder={}",
                model_dir.join("decoder.int8.onnx").display()
            ))
            .arg(format!(
                "--joiner={}",
                model_dir.join("joiner.int8.onnx").display()
            ))
            .arg(format!("--port={}", port))
            .arg(format!("--num-threads={}", num_threads))
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

        let mut child = command.spawn().map_err(|e| {
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
        let samples = wav_to_float32(audio_data)?;

        if is_silent(&samples) {
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
        Ok(trimmed)
    }

    fn service_name(&self) -> &'static str {
        "Parakeet (sherpa-onnx)"
    }
}

/// Resolves the path to the sherpa-onnx binary, downloading it if necessary.
async fn resolve_or_download_binary() -> Result<PathBuf, TranscriptionError> {
    let bin_dir = binary_dir()?;
    let binary_name = binary_name();
    let binary_path = bin_dir.join(&binary_name);

    if binary_path.exists() {
        return Ok(binary_path);
    }

    crate::log_info!("Parakeet binary not found, downloading...");
    download_binary(&bin_dir).await?;

    if !binary_path.exists() {
        return Err(TranscriptionError::Model(format!(
            "Binary downloaded but {} not found at expected path",
            binary_name
        )));
    }

    Ok(binary_path)
}

async fn download_binary(target_dir: &std::path::Path) -> Result<(), TranscriptionError> {
    let archive_name = archive_name();
    let url = format!(
        "https://github.com/k2-fsa/sherpa-onnx/releases/download/v{}/{}",
        BINARY_VERSION, archive_name
    );

    let response = reqwest::get(&url)
        .await
        .map_err(|e| TranscriptionError::Network(format!("Binary download failed: {}", e)))?;

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_pct: f64 = -1.0;

    let archive_path = target_dir.join(archive_name);
    {
        let mut file = tokio::fs::File::create(&archive_path)
            .await
            .map_err(|e| TranscriptionError::Model(format!("Failed to create archive: {}", e)))?;
        use tokio::io::AsyncWriteExt;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| TranscriptionError::Network(e.to_string()))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| TranscriptionError::Model(e.to_string()))?;
            downloaded += chunk.len() as u64;
            if total > 0 {
                let pct = (downloaded as f64 / total as f64) * 100.0;
                if pct - last_pct >= 1.0 {
                    crate::log_info!("Parakeet binary download: {:.0}%", pct);
                    last_pct = pct;
                }
            }
        }
        file.flush()
            .await
            .map_err(|e| TranscriptionError::Model(e.to_string()))?;
    }

    let archive_file = std::fs::File::open(&archive_path)
        .map_err(|e| TranscriptionError::Model(format!("Failed to open archive: {}", e)))?;
    let decoder = bzip2::read::BzDecoder::new(archive_file);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| TranscriptionError::Model(format!("Failed to read tar entries: {}", e)))?
    {
        let mut entry = entry
            .map_err(|e| TranscriptionError::Model(format!("Failed to read tar entry: {}", e)))?;
        let path = entry
            .path()
            .map_err(|_| TranscriptionError::Model("Invalid path in tar".to_string()))?;

        let components: Vec<_> = path.components().collect();
        if components.len() < 2 {
            continue;
        }
        let filename = components[components.len() - 1];
        let out_path = target_dir.join(filename);

        if entry.header().entry_type().is_symlink() {
            #[cfg(unix)]
            {
                let target = entry
                    .link_name()
                    .map_err(|_| TranscriptionError::Model("Invalid symlink target".to_string()))?
                    .ok_or_else(|| {
                        TranscriptionError::Model("Symlink with no target".to_string())
                    })?;
                std::os::unix::fs::symlink(&target, &out_path).map_err(|e| {
                    TranscriptionError::Model(format!("Failed to create symlink: {}", e))
                })?;
            }
        } else if entry.header().entry_type().is_dir() {
            continue;
        } else {
            let mut out_file = std::fs::File::create(&out_path)
                .map_err(|e| TranscriptionError::Model(format!("Failed to create file: {}", e)))?;
            std::io::copy(&mut entry, &mut out_file)
                .map_err(|e| TranscriptionError::Model(format!("Failed to extract file: {}", e)))?;
        }
    }

    let bin_name = binary_name();
    let bin_path = target_dir.join(&bin_name);
    if !bin_path.exists() {
        return Err(TranscriptionError::Model(format!(
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
    crate::log_info!("Parakeet binary downloaded and extracted");
    Ok(())
}

fn binary_dir() -> Result<PathBuf, TranscriptionError> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| TranscriptionError::Model("Could not find config directory".into()))?
        .join("foss-voquill")
        .join("models")
        .join("parakeet")
        .join("bin");
    std::fs::create_dir_all(&config_dir)
        .map_err(|e| TranscriptionError::Model(format!("Failed to create bin dir: {}", e)))?;
    Ok(config_dir)
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

    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        tokio::time::sleep(Duration::from_millis(100)).await;
        match tokio::time::timeout(Duration::from_millis(200), lines.next_line()).await {
            Ok(Ok(Some(line))) => {
                if line.contains("Listening on:") {
                    return Ok(());
                }
            }
            Ok(Ok(None)) => return Err("Process exited before ready".into()),
            Ok(Err(e)) => return Err(format!("Error reading stderr: {}", e)),
            Err(_) => {}
        }
    }
    Err("Startup timeout: sidecar did not become ready within 60s".into())
}

async fn find_free_port(start: u16, end: u16) -> Result<u16, TranscriptionError> {
    for port in start..=end {
        if TcpStream::connect(format!("127.0.0.1:{}", port))
            .await
            .is_err()
        {
            return Ok(port);
        }
    }
    Err(TranscriptionError::Model(
        "No free ports available (6006-6029)".into(),
    ))
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
