use futures_util::StreamExt;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

const SEGMENTATION_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2";
const SEGMENTATION_DIR: &str = "sherpa-onnx-pyannote-segmentation-3-0";
const SEGMENTATION_FILE: &str = "model.onnx";

const EMBEDDING_URL: &str =
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx";
const EMBEDDING_FILE: &str = "3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx";

#[derive(Clone, Debug, serde::Serialize)]
pub struct DiarizationModelDownloadProgress {
    pub model_id: String,
    pub phase: String,
    pub progress: f64,
}

pub fn is_diarization_ready() -> bool {
    let Ok(runner_dir) = crate::paths::python_runner_dir() else {
        return false;
    };
    let models_dir = runner_dir.join("models");
    let seg_path = models_dir.join(SEGMENTATION_DIR).join(SEGMENTATION_FILE);
    let emb_path = models_dir.join(EMBEDDING_FILE);
    seg_path.exists() && emb_path.exists()
}

pub async fn download_diarization_models(app_handle: &AppHandle) -> Result<(), String> {
    let runner_dir = crate::paths::python_runner_dir()?;
    let models_dir = runner_dir.join("models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

    let seg_path = models_dir.join(SEGMENTATION_DIR).join(SEGMENTATION_FILE);
    let emb_path = models_dir.join(EMBEDDING_FILE);

    if seg_path.exists() && emb_path.exists() {
        return Ok(());
    }

    let report = |model_id: &str, phase: &str, progress: f64| {
        let _ = app_handle.emit(
            "diarization-model-download-progress",
            DiarizationModelDownloadProgress {
                model_id: model_id.to_string(),
                phase: phase.to_string(),
                progress,
            },
        );
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // 1. Download & extract segmentation model if missing
    if !seg_path.exists() {
        report("segmentation", "downloading", 0.0);
        crate::log_info!(
            "Downloading speaker segmentation model from {}",
            SEGMENTATION_URL
        );

        let response = client
            .get(SEGMENTATION_URL)
            .send()
            .await
            .map_err(|e| format!("Segmentation model download request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Segmentation model download returned HTTP {}",
                response.status()
            ));
        }

        let total_size = response.content_length().unwrap_or(15 * 1024 * 1024);
        let archive_path = models_dir.join(format!("{}.tar.bz2", SEGMENTATION_DIR));

        let mut downloaded: u64 = 0;
        let mut last_progress: f64 = -1.0;

        {
            let mut file = tokio::fs::File::create(&archive_path)
                .await
                .map_err(|e| format!("Failed to create archive file: {}", e))?;
            let mut stream = response.bytes_stream();
            while let Some(chunk_res) = stream.next().await {
                let chunk = chunk_res.map_err(|e| format!("Download stream error: {}", e))?;
                file.write_all(&chunk)
                    .await
                    .map_err(|e| format!("Failed to write archive: {}", e))?;
                downloaded += chunk.len() as u64;
                let pct = (downloaded as f64 / total_size as f64) * 100.0;
                if pct - last_progress >= 1.0 || pct >= 100.0 {
                    report("segmentation", "downloading", pct);
                    last_progress = pct;
                }
            }
            file.flush()
                .await
                .map_err(|e| format!("Failed to flush archive: {}", e))?;
        }

        report("segmentation", "extracting", 100.0);
        crate::log_info!(
            "Extracting segmentation model archive {}",
            archive_path.display()
        );

        let target_seg_dir = models_dir.join(SEGMENTATION_DIR);
        let extract_res = crate::archive::extract_archive(
            &archive_path,
            &target_seg_dir,
            crate::archive::ExtractLayout::PreservePaths,
        );

        let _ = std::fs::remove_file(&archive_path);
        extract_res.map_err(|e| format!("Failed to extract segmentation model: {}", e))?;

        if !seg_path.exists() {
            return Err(format!(
                "Segmentation model extracted but file missing at {}",
                seg_path.display()
            ));
        }
    }

    // 2. Download embedding model if missing
    if !emb_path.exists() {
        report("embedding", "downloading", 0.0);
        crate::log_info!("Downloading speaker embedding model from {}", EMBEDDING_URL);

        let response = client
            .get(EMBEDDING_URL)
            .send()
            .await
            .map_err(|e| format!("Embedding model download request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Embedding model download returned HTTP {}",
                response.status()
            ));
        }

        let total_size = response.content_length().unwrap_or(90 * 1024 * 1024);
        let mut downloaded: u64 = 0;
        let mut last_progress: f64 = -1.0;

        {
            let mut file = tokio::fs::File::create(&emb_path)
                .await
                .map_err(|e| format!("Failed to create embedding model file: {}", e))?;
            let mut stream = response.bytes_stream();
            while let Some(chunk_res) = stream.next().await {
                let chunk = chunk_res.map_err(|e| format!("Download stream error: {}", e))?;
                file.write_all(&chunk)
                    .await
                    .map_err(|e| format!("Failed to write embedding model: {}", e))?;
                downloaded += chunk.len() as u64;
                let pct = (downloaded as f64 / total_size as f64) * 100.0;
                if pct - last_progress >= 0.5 || pct >= 100.0 {
                    report("embedding", "downloading", pct);
                    last_progress = pct;
                }
            }
            file.flush()
                .await
                .map_err(|e| format!("Failed to flush embedding model: {}", e))?;
        }

        report("embedding", "downloading", 100.0);
    }

    crate::log_info!("Diarization models ready");
    Ok(())
}
