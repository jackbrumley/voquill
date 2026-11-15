//! The parakeet.cpp HTTP server: runs Parakeet GGUF models on the graphics
//! card through ggml (Vulkan on Windows/Linux, Metal on macOS) for the
//! engine "Parakeet (GPU)".

use super::SAMPLE_RATE;
use crate::transcription::TranscriptionError;
use std::path::{Path, PathBuf};
use std::time::Duration;

const RELEASE_VERSION: &str = "0.6.1";
const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(300);
/// The stderr line the server prints once it accepts requests.
pub(super) const READY_MARKER: &str = "listening on";
pub(super) const LOG_LABEL: &str = "parakeet.cpp";
/// The stderr line naming the ggml device the model runs on, e.g.
/// `[parakeet] pk::Backend using device: Vulkan0`.
const DEVICE_MARKER: &str = "using device:";

pub(super) fn download_spec() -> Result<crate::sidecar::SidecarDownload, TranscriptionError> {
    let archive_name = archive_name()?;
    Ok(crate::sidecar::SidecarDownload {
        log_label: LOG_LABEL,
        archive_url: format!(
            "https://github.com/mudler/parakeet.cpp/releases/download/v{}/{}",
            RELEASE_VERSION, archive_name
        ),
        archive_name,
        layout: crate::archive::ExtractLayout::Flat,
    })
}

pub(super) fn binary_dir() -> Result<PathBuf, String> {
    Ok(crate::paths::models_dir()?
        .join("transcription")
        .join("parakeet-cpp")
        .join("bin"))
}

pub(super) fn binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "parakeet-server.exe"
    } else {
        "parakeet-server"
    }
}

/// The GPU build for this platform. parakeet.cpp ships no GPU build for
/// Intel Macs or other architectures, so the GPU engine is unavailable there.
fn archive_name() -> Result<String, TranscriptionError> {
    let variant = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-vulkan-x64.tar.gz",
        ("linux", "aarch64") => "linux-vulkan-arm64.tar.gz",
        ("windows", "x86_64") => "win-vulkan-x64.zip",
        ("macos", "aarch64") => "macos-metal-arm64.tar.gz",
        (os, arch) => {
            return Err(TranscriptionError::Model(format!(
                "Parakeet (GPU) is not available on {}-{}. Choose Parakeet or Whisper.cpp instead.",
                os, arch
            )))
        }
    };
    Ok(format!("parakeet-v{}-bin-{}", RELEASE_VERSION, variant))
}

pub(super) fn args(model_file: &Path, port: u16) -> Vec<String> {
    vec![
        "--model".to_string(),
        model_file.display().to_string(),
        "--host".to_string(),
        "127.0.0.1".to_string(),
        "--port".to_string(),
        port.to_string(),
    ]
}

/// The device named by a server log line, if the line reports one.
pub(super) fn device_from_log_line(line: &str) -> Option<String> {
    line.split_once(DEVICE_MARKER)
        .map(|(_, device)| device.trim().to_string())
        .filter(|device| !device.is_empty())
}

/// Whether a ggml device name is a graphics card rather than the CPU.
pub(super) fn is_gpu_device(device: &str) -> bool {
    ["Vulkan", "Metal", "CUDA"]
        .iter()
        .any(|prefix| device.starts_with(prefix))
}

/// Transcribes one piece of audio with an OpenAI-style WAV upload.
pub(super) async fn transcribe_piece(
    port: u16,
    samples: &[f32],
) -> Result<String, TranscriptionError> {
    let wav = encode_wav(samples)?;
    let part = reqwest::multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| TranscriptionError::Network(e.to_string()))?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("response_format", "json");

    let response = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{}/v1/audio/transcriptions", port))
        .multipart(form)
        .timeout(TRANSCRIBE_TIMEOUT)
        .send()
        .await
        .map_err(|e| TranscriptionError::Network(format!("parakeet.cpp request failed: {}", e)))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| TranscriptionError::Network(format!("parakeet.cpp response failed: {}", e)))?;
    if !status.is_success() {
        return Err(TranscriptionError::Model(format!(
            "parakeet.cpp returned {}: {}",
            status,
            body.trim()
        )));
    }

    let parsed: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        TranscriptionError::Model(format!("parakeet.cpp sent an unreadable response: {}", e))
    })?;
    let text = parsed
        .get("text")
        .and_then(|value| value.as_str())
        .ok_or_else(|| TranscriptionError::Model("parakeet.cpp response has no text".into()))?;
    Ok(text.trim().to_string())
}

fn encode_wav(samples: &[f32]) -> Result<Vec<u8>, TranscriptionError> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)
            .map_err(|e| TranscriptionError::Audio(e.to_string()))?;
        for &sample in samples {
            let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer
                .write_sample(value)
                .map_err(|e| TranscriptionError::Audio(e.to_string()))?;
        }
        writer
            .finalize()
            .map_err(|e| TranscriptionError::Audio(e.to_string()))?;
    }
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_is_read_from_the_backend_log_line() {
        assert_eq!(
            device_from_log_line("[parakeet] pk::Backend using device: Vulkan0"),
            Some("Vulkan0".to_string())
        );
        assert_eq!(device_from_log_line("parakeet-server: listening on"), None);
    }

    #[test]
    fn cpu_device_is_not_a_gpu() {
        assert!(is_gpu_device("Vulkan0"));
        assert!(is_gpu_device("Metal"));
        assert!(!is_gpu_device("CPU"));
    }
}
