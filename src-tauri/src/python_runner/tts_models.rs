use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone)]
pub struct TtsModelMeta {
    pub key: &'static str,
    pub label: &'static str,
    pub archive: &'static str,
    pub url: &'static str,
    pub model_file: &'static str,
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

/// The files a base voice is made of, under the runner's model folder.
#[derive(Clone, Debug)]
pub struct VoiceFiles {
    pub voice_dir: PathBuf,
    pub model_path: PathBuf,
    pub tokens_path: PathBuf,
    pub data_dir: PathBuf,
}

impl VoiceFiles {
    pub fn are_present(&self) -> bool {
        self.model_path.exists() && self.tokens_path.exists() && self.data_dir.exists()
    }
}

fn base_model(model_key: &str) -> Result<&'static TtsModelMeta, String> {
    TTS_BASE_MODELS
        .iter()
        .find(|meta| meta.key == model_key)
        .ok_or_else(|| format!("Unknown voice model '{}'", model_key))
}

/// Where a base voice's files live (whether or not they are downloaded).
pub fn voice_files(model_key: &str) -> Result<VoiceFiles, String> {
    let meta = base_model(model_key)?;
    let voice_dir = crate::paths::python_runner_dir()?
        .join("models")
        .join("tts")
        .join(meta.archive);
    Ok(VoiceFiles {
        model_path: voice_dir.join(meta.model_file),
        tokens_path: voice_dir.join("tokens.txt"),
        data_dir: voice_dir.join("espeak-ng-data"),
        voice_dir,
    })
}

/// A base voice as the Voice Studio lists it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct BaseVoiceModelInfo {
    pub id: String,
    pub label: String,
    pub is_multi_speaker: bool,
    pub is_ready: bool,
}

/// Every base voice, from this catalog (the owner of voice facts), with
/// whether its files are on disk. Needs no Python runner.
pub fn base_voice_models() -> Vec<BaseVoiceModelInfo> {
    TTS_BASE_MODELS
        .iter()
        .map(|meta| BaseVoiceModelInfo {
            id: meta.key.to_string(),
            label: meta.label.to_string(),
            is_multi_speaker: meta.is_multi_speaker,
            is_ready: is_tts_model_downloaded(meta.key),
        })
        .collect()
}

pub fn is_tts_model_downloaded(model_key: &str) -> bool {
    voice_files(model_key).is_ok_and(|files| files.are_present())
}

/// Downloads and extracts a base voice (by its model key) unless its files
/// are already present.
pub async fn download_tts_model(
    app_handle: &AppHandle,
    model_key: &str,
) -> Result<PathBuf, String> {
    let meta = base_model(model_key)?;
    let files = voice_files(model_key)?;
    if files.are_present() {
        return Ok(files.voice_dir);
    }
    let VoiceFiles {
        voice_dir,
        model_path,
        ..
    } = files;
    let tts_models_dir = voice_dir
        .parent()
        .ok_or_else(|| format!("Voice folder {} has no parent", voice_dir.display()))?
        .to_path_buf();
    std::fs::create_dir_all(&tts_models_dir).map_err(|e| e.to_string())?;

    let report = |phase: &str, progress: f64| {
        let _ = app_handle.emit(
            "tts-model-download-progress",
            TtsModelDownloadProgress {
                model_id: model_key.to_string(),
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
