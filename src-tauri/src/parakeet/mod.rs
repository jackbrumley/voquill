//! NVIDIA Parakeet transcription through a local sidecar server.
//!
//! Two servers run Parakeet: sherpa-onnx on the CPU (engine "Parakeet") and
//! parakeet.cpp on the graphics card (engine "Parakeet (GPU)"). Each backend
//! module owns its release archive, arguments and wire protocol; this module
//! owns what they share: starting and watching the process, splitting long
//! recordings, and the `TranscriptionService` interface.

mod cache;
mod parakeet_cpp;
mod sherpa_onnx;

pub use cache::ParakeetSidecarCache;

use crate::transcription::{TranscriptionError, TranscriptionService};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Child;
use tokio::sync::Mutex;

const PORT_START: u16 = 6006;
const PORT_END: u16 = 6080;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
const SAMPLE_RATE: u32 = 16000;
/// Longest piece of audio sent to the sidecar in one request. sherpa-onnx
/// rejects anything over its `--max-utterance-length` (300 s by default),
/// parakeet.cpp fails on inputs over 5 minutes, and ONNX Runtime keeps the
/// working memory of the longest piece it has decoded for the life of the
/// process (measured with the 0.6B int8 model: 60 s pieces leave it ~350 MB
/// larger than 30 s pieces, at the same speed), so recordings are split at
/// pauses into pieces of at most this length.
const MAX_PIECE_SECONDS: f32 = 30.0;

/// Which server runs the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParakeetBackend {
    /// sherpa-onnx with int8 ONNX files, on the CPU.
    SherpaOnnx,
    /// parakeet.cpp with a GGUF file, on the graphics card.
    ParakeetCpp,
}

impl ParakeetBackend {
    fn log_label(self) -> &'static str {
        match self {
            Self::SherpaOnnx => sherpa_onnx::LOG_LABEL,
            Self::ParakeetCpp => parakeet_cpp::LOG_LABEL,
        }
    }

    fn ready_marker(self) -> &'static str {
        match self {
            Self::SherpaOnnx => sherpa_onnx::READY_MARKER,
            Self::ParakeetCpp => parakeet_cpp::READY_MARKER,
        }
    }

    async fn ensure_binary(self) -> Result<PathBuf, TranscriptionError> {
        let (bin_dir, binary_name, download) = match self {
            Self::SherpaOnnx => (
                sherpa_onnx::binary_dir(),
                sherpa_onnx::binary_name(),
                sherpa_onnx::download_spec(),
            ),
            Self::ParakeetCpp => (
                parakeet_cpp::binary_dir(),
                parakeet_cpp::binary_name(),
                parakeet_cpp::download_spec()?,
            ),
        };
        let bin_dir = bin_dir.map_err(TranscriptionError::Model)?;
        crate::sidecar::ensure_binary(bin_dir, binary_name, download)
            .await
            .map_err(|e| TranscriptionError::Model(e.to_string()))
    }

    fn args(self, model_path: &Path, port: u16, num_threads: usize) -> Vec<String> {
        match self {
            Self::SherpaOnnx => sherpa_onnx::args(model_path, port, num_threads),
            Self::ParakeetCpp => parakeet_cpp::args(model_path, port),
        }
    }

    async fn transcribe_piece(
        self,
        port: u16,
        samples: &[f32],
    ) -> Result<String, TranscriptionError> {
        match self {
            Self::SherpaOnnx => sherpa_onnx::transcribe_piece(port, samples).await,
            Self::ParakeetCpp => parakeet_cpp::transcribe_piece(port, samples).await,
        }
    }
}

/// A running Parakeet sidecar. Clones share the same process, which is
/// killed when the last clone is dropped.
#[derive(Clone)]
pub struct ParakeetService {
    port: u16,
    process: Arc<Mutex<Option<Child>>>,
    model_size: String,
    backend: ParakeetBackend,
    /// The compute device the server reported (parakeet.cpp only).
    device: Option<String>,
}

impl ParakeetService {
    /// Starts the server for `model_path`: the model directory for
    /// sherpa-onnx, the GGUF file for parakeet.cpp.
    pub async fn new(
        backend: ParakeetBackend,
        model_path: PathBuf,
        model_size: &str,
        num_threads: usize,
    ) -> Result<Self, TranscriptionError> {
        let binary_path = backend.ensure_binary().await?;
        let port = crate::sidecar::find_free_port(PORT_START, PORT_END)
            .map_err(|e| TranscriptionError::Model(e.to_string()))?;
        let args = backend.args(&model_path, port, num_threads);
        let label = backend.log_label();

        let mut child = crate::sidecar::spawn_sidecar(&binary_path, &args)
            .map_err(|e| TranscriptionError::Model(format!("Failed to spawn {}: {}", label, e)))?;

        let stderr = child.stderr.take().unwrap();
        let mut stderr_reader = tokio::io::BufReader::new(stderr);
        let startup_lines =
            match wait_for_ready(&mut stderr_reader, backend.ready_marker(), label).await {
                Ok(lines) => lines,
                Err(e) => {
                    let _ = child.kill().await;
                    return Err(TranscriptionError::Model(format!(
                        "Parakeet sidecar ({}) failed to start: {}",
                        label, e
                    )));
                }
            };

        // Drain stderr in background so the server doesn't get SIGPIPE.
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut lines = stderr_reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                crate::log_info!("{}: {}", label, line);
            }
        });

        let device = match backend {
            ParakeetBackend::SherpaOnnx => None,
            ParakeetBackend::ParakeetCpp => startup_lines
                .iter()
                .find_map(|line| parakeet_cpp::device_from_log_line(line)),
        };
        crate::log_info!(
            "Parakeet sidecar ({}) started on port {}{}",
            label,
            port,
            device
                .as_deref()
                .map(|d| format!(", device {}", d))
                .unwrap_or_default()
        );

        let service = Self {
            port,
            process: Arc::new(Mutex::new(Some(child))),
            model_size: model_size.to_string(),
            backend,
            device,
        };
        service.warm_up().await;
        Ok(service)
    }

    /// The first GPU request compiles shaders (~0.5 s); paying that at start
    /// keeps it out of the first dictation.
    async fn warm_up(&self) {
        if self.backend != ParakeetBackend::ParakeetCpp {
            return;
        }
        let one_second_of_silence = vec![0.0_f32; SAMPLE_RATE as usize];
        if let Err(e) = self
            .backend
            .transcribe_piece(self.port, &one_second_of_silence)
            .await
        {
            crate::log_warn!("Parakeet GPU warm-up request failed: {}", e);
        }
    }

    /// Why the GPU backend is not using a graphics card, if it isn't.
    /// parakeet.cpp silently falls back to the CPU when no GPU starts.
    pub fn gpu_fallback_reason(&self) -> Option<String> {
        if self.backend != ParakeetBackend::ParakeetCpp {
            return None;
        }
        match self.device.as_deref() {
            Some(device) if parakeet_cpp::is_gpu_device(device) => None,
            Some(device) => Some(format!(
                "parakeet.cpp found no usable graphics card and is running on {}",
                device
            )),
            None => Some("parakeet.cpp did not report which device it is using".to_string()),
        }
    }

    /// Whether the sidecar process is still running, so a cached service
    /// whose process crashed or was killed is not reused.
    pub async fn is_alive(&self) -> bool {
        let mut process = self.process.lock().await;
        match process.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    }

    /// Whether `other` is a clone of this service, i.e. drives the same process.
    pub fn shares_process_with(&self, other: &ParakeetService) -> bool {
        Arc::ptr_eq(&self.process, &other.process)
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

        let pieces =
            crate::audio::chunking::split_at_pauses(&samples, SAMPLE_RATE, MAX_PIECE_SECONDS);
        if pieces.len() > 1 {
            crate::log_info!(
                "Parakeet: splitting {:.1}s of audio into {} pieces of at most {}s",
                samples.len() as f32 / SAMPLE_RATE as f32,
                pieces.len(),
                MAX_PIECE_SECONDS
            );
        }

        let mut piece_texts = Vec::with_capacity(pieces.len());
        for piece in pieces {
            let piece_samples = &samples[piece];
            if is_silent(piece_samples) {
                continue;
            }
            let piece_text = self
                .backend
                .transcribe_piece(self.port, piece_samples)
                .await?;
            if !piece_text.is_empty() {
                piece_texts.push(piece_text);
            }
        }
        let text = piece_texts.join(" ");
        crate::log_info!(
            "Transcription ({}, engine=Parakeet via {}): inference={:?}, chars={}",
            self.model_size,
            self.backend.log_label(),
            start_time.elapsed(),
            text.len()
        );
        Ok(text)
    }

    fn service_name(&self) -> &'static str {
        match self.backend {
            ParakeetBackend::SherpaOnnx => "Parakeet (sherpa-onnx)",
            ParakeetBackend::ParakeetCpp => "Parakeet (parakeet.cpp)",
        }
    }
}

/// Reads the server's stderr until it prints `ready_marker`, returning every
/// line seen before it (startup diagnostics such as the compute device).
async fn wait_for_ready(
    reader: &mut tokio::io::BufReader<tokio::process::ChildStderr>,
    ready_marker: &str,
    label: &str,
) -> Result<Vec<String>, String> {
    use tokio::io::AsyncBufReadExt;
    let mut lines = reader.lines();
    let mut captured_lines = Vec::new();

    let start = std::time::Instant::now();
    while start.elapsed() < STARTUP_TIMEOUT {
        match tokio::time::timeout(Duration::from_millis(200), lines.next_line()).await {
            Ok(Ok(Some(line))) => {
                crate::log_info!("{}: {}", label, line);
                if line.contains(ready_marker) {
                    return Ok(captured_lines);
                }
                captured_lines.push(line);
            }
            Ok(Ok(None)) => {
                let error_detail = if captured_lines.is_empty() {
                    "no output".to_string()
                } else {
                    captured_lines.join(" | ")
                };
                return Err(format!("Process exited before ready: {}", error_detail));
            }
            Ok(Err(e)) => return Err(format!("Error reading stderr: {}", e)),
            Err(_) => {}
        }
    }
    Err(format!(
        "Startup timeout: sidecar did not become ready within {}s",
        STARTUP_TIMEOUT.as_secs()
    ))
}

fn wav_to_float32(wav_bytes: &[u8]) -> Result<Vec<f32>, TranscriptionError> {
    let mut reader = hound::WavReader::new(std::io::Cursor::new(wav_bytes))
        .map_err(|e| TranscriptionError::Audio(e.to_string()))?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != SAMPLE_RATE {
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
