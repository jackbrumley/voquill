use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone)]
pub struct TtsModelMeta {
    pub key: &'static str,
    #[allow(dead_code)]
    pub label: &'static str,
    pub archive: &'static str,
    pub url: &'static str,
    pub model_file: &'static str,
    #[allow(dead_code)]
    pub is_multi_speaker: bool,
}

pub const TTS_BASE_MODELS: &[TtsModelMeta] = &[
    TtsModelMeta {
        key: "piper-en_GB-northern_english_male-medium",
        label: "Northern English Male (SAS Price / Tactical)",
        archive: "vits-piper-en_GB-northern_english_male-medium",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_GB-northern_english_male-medium.tar.bz2",
        model_file: "en_GB-northern_english_male-medium.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_GB-alan-medium",
        label: "Alan (Cold British Commander / Dark Baritone)",
        archive: "vits-piper-en_GB-alan-medium",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_GB-alan-medium.tar.bz2",
        model_file: "en_GB-alan-medium.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_US-norman-medium",
        label: "Norman (Deep American Baritone / Dispatcher)",
        archive: "vits-piper-en_US-norman-medium",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-norman-medium.tar.bz2",
        model_file: "en_US-norman-medium.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_US-joe-medium",
        label: "Joe (Gritty Older Combat Veteran)",
        archive: "vits-piper-en_US-joe-medium",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-joe-medium.tar.bz2",
        model_file: "en_US-joe-medium.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_US-bryce-medium",
        label: "Bryce (High-Energy / Commanding Operator)",
        archive: "vits-piper-en_US-bryce-medium",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-bryce-medium.tar.bz2",
        model_file: "en_US-bryce-medium.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_US-danny-low",
        label: "Danny (Fast Tactical Field Operator)",
        archive: "vits-piper-en_US-danny-low",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-danny-low.tar.bz2",
        model_file: "en_US-danny-low.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_US-ryan-low",
        label: "Ryan (Deep Male / Titan Base)",
        archive: "vits-piper-en_US-ryan-low",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-ryan-low.tar.bz2",
        model_file: "en_US-ryan-low.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_US-amy-low",
        label: "Amy (Cyberpunk EVA / Clear Sci-Fi Female)",
        archive: "vits-piper-en_US-amy-low",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-amy-low.tar.bz2",
        model_file: "en_US-amy-low.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_GB-cori-medium",
        label: "Cori (Expressive British Female)",
        archive: "vits-piper-en_GB-cori-medium",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_GB-cori-medium.tar.bz2",
        model_file: "en_GB-cori-medium.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_US-glados",
        label: "GLaDOS (Iconic Robotic Portal AI)",
        archive: "vits-piper-en_US-glados",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-glados.tar.bz2",
        model_file: "en_US-glados.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_GB-southern_english_female-low",
        label: "Southern English Female (Flight Deck ATC)",
        archive: "vits-piper-en_GB-southern_english_female-low",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_GB-southern_english_female-low.tar.bz2",
        model_file: "en_GB-southern_english_female-low.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_US-lessac-low",
        label: "Nova Studio (Clean Female Narration)",
        archive: "vits-piper-en_US-lessac-low",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-lessac-low.tar.bz2",
        model_file: "en_US-lessac-low.onnx",
        is_multi_speaker: false,
    },
    TtsModelMeta {
        key: "piper-en_US-libritts_r-medium",
        label: "LibriTTS-R Multi-Speaker (904 Speakers)",
        archive: "vits-piper-en_US-libritts_r-medium",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-libritts_r-medium.tar.bz2",
        model_file: "en_US-libritts_r-medium.onnx",
        is_multi_speaker: true,
    },
];

#[derive(Clone, Debug, serde::Serialize)]
pub struct TtsModelDownloadProgress {
    pub model_id: String,
    pub phase: String,
    pub progress: f64,
}

pub fn resolve_base_model_key(voice_or_model_id: &str) -> &'static str {
    match voice_or_model_id {
        "tactical-comms" | "piper-en_US-arctic-medium" => {
            "piper-en_GB-northern_english_male-medium"
        }
        "titan-mech" | "nanosuit" | "apex-studio" | "piper-en_US-ryan-low" => {
            "piper-en_US-ryan-low"
        }
        "glados" | "piper-en_US-glados" => "piper-en_US-glados",
        "cyberpunk-eva" | "piper-en_US-amy-low" => "piper-en_US-amy-low",
        "flight-deck" | "piper-en_GB-southern_english_female-low" => {
            "piper-en_GB-southern_english_female-low"
        }
        "nova-studio" | "piper-en_US-lessac-low" => "piper-en_US-lessac-low",
        other => {
            if let Some(m) = TTS_BASE_MODELS.iter().find(|m| m.key == other) {
                return m.key;
            }
            if let Ok(presets_file) = crate::paths::voice_presets_file() {
                if presets_file.exists() {
                    if let Ok(data) = std::fs::read_to_string(presets_file) {
                        if let Ok(presets) = serde_json::from_str::<Vec<serde_json::Value>>(&data) {
                            for p in presets {
                                if p.get("id").and_then(|v| v.as_str()) == Some(other) {
                                    if let Some(mk) = p.get("model_key").and_then(|v| v.as_str()) {
                                        if let Some(found) =
                                            TTS_BASE_MODELS.iter().find(|m| m.key == mk)
                                        {
                                            return found.key;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "piper-en_GB-northern_english_male-medium"
        }
    }
}

pub fn is_tts_model_downloaded(model_key: &str) -> bool {
    let base_key = resolve_base_model_key(model_key);
    let Some(meta) = TTS_BASE_MODELS.iter().find(|m| m.key == base_key) else {
        return false;
    };
    let Ok(runner_dir) = crate::paths::python_runner_dir() else {
        return false;
    };
    let voice_dir = runner_dir.join("models").join("tts").join(meta.archive);
    let model_path = voice_dir.join(meta.model_file);
    let tokens_path = voice_dir.join("tokens.txt");
    let data_dir = voice_dir.join("espeak-ng-data");
    model_path.exists() && tokens_path.exists() && data_dir.exists()
}

pub async fn download_tts_model(
    app_handle: &AppHandle,
    voice_or_model_id: &str,
) -> Result<PathBuf, String> {
    let base_key = resolve_base_model_key(voice_or_model_id);
    let meta = TTS_BASE_MODELS
        .iter()
        .find(|m| m.key == base_key)
        .ok_or_else(|| format!("Unknown TTS model: {}", base_key))?;

    let runner_dir = crate::paths::python_runner_dir()?;
    let tts_models_dir = runner_dir.join("models").join("tts");
    std::fs::create_dir_all(&tts_models_dir).map_err(|e| e.to_string())?;

    let voice_dir = tts_models_dir.join(meta.archive);
    let model_path = voice_dir.join(meta.model_file);
    let tokens_path = voice_dir.join("tokens.txt");
    let data_dir = voice_dir.join("espeak-ng-data");

    if model_path.exists() && tokens_path.exists() && data_dir.exists() {
        return Ok(voice_dir);
    }

    let report = |phase: &str, progress: f64| {
        let _ = app_handle.emit(
            "tts-model-download-progress",
            TtsModelDownloadProgress {
                model_id: voice_or_model_id.to_string(),
                phase: phase.to_string(),
                progress,
            },
        );
    };

    report("downloading", 0.0);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    crate::log_info!("Downloading TTS model {} from {}", meta.key, meta.url);
    let response = client
        .get(meta.url)
        .send()
        .await
        .map_err(|e| format!("TTS model download request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "TTS model download returned HTTP {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(30 * 1024 * 1024);
    let archive_path = tts_models_dir.join(format!("{}.tar.bz2", meta.archive));

    let mut downloaded: u64 = 0;
    let mut last_progress: f64 = -1.0;

    {
        let mut file = tokio::fs::File::create(&archive_path)
            .await
            .map_err(|e| format!("Failed to create archive file: {}", e))?;
        use futures_util::StreamExt;
        use tokio::io::AsyncWriteExt;
        let mut stream = response.bytes_stream();
        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res.map_err(|e| format!("Download stream error: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write archive: {}", e))?;
            downloaded += chunk.len() as u64;
            let pct = (downloaded as f64 / total_size as f64) * 100.0;
            if pct - last_progress >= 0.5 || pct >= 100.0 {
                report("downloading", pct);
                last_progress = pct;
            }
        }
        file.flush()
            .await
            .map_err(|e| format!("Failed to flush archive: {}", e))?;
    }

    report("extracting", 100.0);
    crate::log_info!("Extracting TTS model archive {}", archive_path.display());

    let extract_res = crate::archive::extract_archive(
        &archive_path,
        &voice_dir,
        crate::archive::ExtractLayout::PreservePaths,
    );

    let _ = std::fs::remove_file(&archive_path);

    extract_res.map_err(|e| format!("Failed to extract TTS model {}: {}", meta.key, e))?;

    if !model_path.exists() {
        return Err(format!(
            "TTS model extracted but model file missing at {}",
            model_path.display()
        ));
    }

    report("downloading", 100.0);
    crate::log_info!("TTS model {} ready at {}", meta.key, voice_dir.display());

    Ok(voice_dir)
}
