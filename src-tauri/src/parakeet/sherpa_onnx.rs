//! The sherpa-onnx offline WebSocket server: runs Parakeet ONNX models on the
//! CPU (engine "Parakeet").

use super::SAMPLE_RATE;
use crate::transcription::TranscriptionError;
use futures_util::{SinkExt, StreamExt};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_tungstenite::connect_async;

const BINARY_VERSION: &str = "1.13.4";
const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(300);
/// Requests the sidecar decodes at once. Voquill sends one piece at a time
/// (a voice macro may overlap a dictation), and each request already uses
/// every inference thread, so a larger pool only holds idle threads.
const DECODE_WORKERS: usize = 2;
/// The stderr line the server prints once it accepts connections.
pub(super) const READY_MARKER: &str = "Listening on:";
pub(super) const LOG_LABEL: &str = "sherpa-onnx";

pub(super) fn download_spec() -> crate::sidecar::SidecarDownload {
    let archive_name = archive_name();
    crate::sidecar::SidecarDownload {
        log_label: LOG_LABEL,
        archive_url: format!(
            "https://github.com/k2-fsa/sherpa-onnx/releases/download/v{}/{}",
            BINARY_VERSION, archive_name
        ),
        archive_name: archive_name.to_string(),
        layout: crate::archive::ExtractLayout::Flat,
    }
}

pub(super) fn binary_dir() -> Result<PathBuf, String> {
    Ok(crate::paths::models_dir()?
        .join("transcription")
        .join("parakeet")
        .join("bin"))
}

pub(super) fn binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "sherpa-onnx-offline-websocket-server.exe"
    } else {
        "sherpa-onnx-offline-websocket-server"
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

/// Server arguments for a model directory holding the int8 ONNX files.
pub(super) fn args(model_dir: &Path, port: u16, num_threads: usize) -> Vec<String> {
    vec![
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
        format!("--num-work-threads={}", DECODE_WORKERS),
        "--log-file=".to_string(),
    ]
}

/// Transcribes one piece of audio over its own WebSocket connection.
pub(super) async fn transcribe_piece(
    port: u16,
    samples: &[f32],
) -> Result<String, TranscriptionError> {
    let url = format!("ws://127.0.0.1:{}", port);
    let (mut ws, _) = connect_async(&url)
        .await
        .map_err(|e| TranscriptionError::Network(format!("WebSocket connect failed: {}", e)))?;

    let num_bytes = (samples.len() * 4) as i32;
    let mut msg = Vec::with_capacity(8 + samples.len() * 4);
    msg.extend_from_slice(&(SAMPLE_RATE as i32).to_le_bytes());
    msg.extend_from_slice(&num_bytes.to_le_bytes());
    for &sample in samples {
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

    Ok(parse_offline_result(&result))
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
