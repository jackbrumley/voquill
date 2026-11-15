mod catalog;

use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub engine: String,
    pub size: String,
    pub file_size: u64,
    pub download_url: String,
    pub sha256: String,
    pub label: String,
    pub description: String,
    pub recommended: bool,
    pub category: String,
}

/// The required model files for a Parakeet / sherpa-onnx model directory.
const PARAKEET_REQUIRED_FILES: &[&str] = &[
    "encoder.int8.onnx",
    "decoder.int8.onnx",
    "joiner.int8.onnx",
    "tokens.txt",
];

/// Moves model files from a nested wrapper directory up into the model root if
/// a prior extraction left them wrapped.
fn ensure_parakeet_model_flattened(dir: &std::path::Path) {
    if !dir.is_dir() {
        return;
    }
    if PARAKEET_REQUIRED_FILES.iter().all(|f| dir.join(f).exists()) {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let subdirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    if subdirs.len() == 1 {
        let sub = &subdirs[0];
        if PARAKEET_REQUIRED_FILES.iter().all(|f| sub.join(f).exists()) {
            crate::log_info!(
                "Self-healing: flattening nested Parakeet model dir from {} into {}",
                sub.display(),
                dir.display()
            );
            if let Ok(children) = std::fs::read_dir(sub) {
                for child in children.flatten() {
                    let dest = dir.join(child.file_name());
                    let _ = std::fs::rename(child.path(), &dest);
                }
            }
            let _ = std::fs::remove_dir(sub);
        }
    }
}

/// Migrates legacy model storage structures to the unified purpose -> engine hierarchy:
/// - models/ggml-*.bin & models/transcription/ggml-*.bin -> models/transcription/whisper/
/// - models/parakeet/ -> models/transcription/parakeet/
/// - models/post-process/*.gguf & models/post-process/bin/ -> models/post-process/llama/
pub fn migrate_legacy_model_paths(models_dir: &std::path::Path) {
    if !models_dir.exists() {
        return;
    }

    let whisper_dir = models_dir.join("transcription").join("whisper");

    // 1. Move root ggml-*.bin -> transcription/whisper/
    if let Ok(entries) = std::fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if file_name.starts_with("ggml-") && file_name.ends_with(".bin") {
                        let _ = std::fs::create_dir_all(&whisper_dir);
                        let dest = whisper_dir.join(file_name);
                        if !dest.exists() {
                            let _ = std::fs::rename(&path, &dest);
                            crate::log_info!(
                                "Migrated model: {} -> {}",
                                path.display(),
                                dest.display()
                            );
                        }
                    }
                }
            }
        }
    }

    // 2. Move transcription/ggml-*.bin (legacy flat transcription folder) -> transcription/whisper/
    let transcription_dir = models_dir.join("transcription");
    if transcription_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&transcription_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.starts_with("ggml-") && file_name.ends_with(".bin") {
                            let _ = std::fs::create_dir_all(&whisper_dir);
                            let dest = whisper_dir.join(file_name);
                            if !dest.exists() {
                                let _ = std::fs::rename(&path, &dest);
                                crate::log_info!(
                                    "Migrated model: {} -> {}",
                                    path.display(),
                                    dest.display()
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Move models/parakeet/ -> models/transcription/parakeet/
    let old_parakeet_dir = models_dir.join("parakeet");
    let new_parakeet_dir = models_dir.join("transcription").join("parakeet");
    if old_parakeet_dir.is_dir() {
        let _ = std::fs::create_dir_all(&new_parakeet_dir);
        if let Ok(entries) = std::fs::read_dir(&old_parakeet_dir) {
            for entry in entries.flatten() {
                let dest = new_parakeet_dir.join(entry.file_name());
                if !dest.exists() {
                    let _ = std::fs::rename(entry.path(), &dest);
                    crate::log_info!(
                        "Migrated parakeet item: {} -> {}",
                        entry.path().display(),
                        dest.display()
                    );
                }
            }
        }
        let _ = std::fs::remove_dir_all(&old_parakeet_dir);
    }

    // 4. Move models/post-process/*.gguf and models/post-process/bin/ -> models/post-process/llama/
    let post_process_dir = models_dir.join("post-process");
    let llama_dir = post_process_dir.join("llama");
    if post_process_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&post_process_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = entry.file_name();
                if file_name == "llama" {
                    continue;
                }
                let _ = std::fs::create_dir_all(&llama_dir);
                let dest = llama_dir.join(&file_name);
                if !dest.exists() {
                    let _ = std::fs::rename(&path, &dest);
                    crate::log_info!(
                        "Migrated post-process item: {} -> {}",
                        path.display(),
                        dest.display()
                    );
                }
            }
        }
    }
}

/// Progress of a model download, reported to the frontend through the
/// `model-download-progress` event.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub phase: DownloadPhase,
    pub progress: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadPhase {
    Downloading,
    Extracting,
}

pub struct ModelManager {
    pub models_dir: PathBuf,
}

impl ModelManager {
    pub fn new() -> Result<Self, String> {
        let models_dir = crate::paths::models_dir()?;
        migrate_legacy_model_paths(&models_dir);
        Ok(Self { models_dir })
    }

    /// The on-disk path for a model, taking its engine into account.
    /// whisper.cpp models are flat files:  models/transcription/whisper/ggml-{size}.bin
    /// Parakeet models are directories:    models/transcription/parakeet/{size}/
    /// Post-process models are flat files: models/post-process/llama/{size}.gguf
    pub fn get_model_path(&self, model: &ModelInfo) -> PathBuf {
        match model.engine.as_str() {
            e if e.contains("Whisper.cpp") => self
                .models_dir
                .join("transcription")
                .join("whisper")
                .join(format!("ggml-{}.bin", model.size)),
            "Parakeet (GPU)" => self
                .models_dir
                .join("transcription")
                .join("parakeet-cpp")
                .join(format!("{}.gguf", model.size)),
            e if e.starts_with("Post-Process") => self
                .models_dir
                .join("post-process")
                .join("llama")
                .join(format!("{}.gguf", model.size)),
            _ => {
                let dir = self
                    .models_dir
                    .join("transcription")
                    .join("parakeet")
                    .join(&model.size);
                ensure_parakeet_model_flattened(&dir);
                dir
            }
        }
    }

    /// Checks whether the model is present on disk. For whisper.cpp this is a
    /// single file check. For Parakeet it checks for all required ONNX files.
    pub fn is_model_downloaded(&self, model: &ModelInfo) -> bool {
        match model.engine.as_str() {
            "Parakeet" => {
                let dir = self.get_model_path(model);
                ensure_parakeet_model_flattened(&dir);
                PARAKEET_REQUIRED_FILES.iter().all(|f| dir.join(f).exists())
            }
            _ => self.get_model_path(model).exists(),
        }
    }

    /// Downloads a model. For whisper.cpp this is a single-file download. For
    /// Parakeet the download is a tar.bz2 archive that gets extracted into a
    /// subdirectory.
    pub async fn download_model<F>(
        &self,
        model: &ModelInfo,
        progress_callback: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(DownloadProgress) + Send + 'static,
    {
        let report = |phase: DownloadPhase, progress: f64| {
            progress_callback(DownloadProgress { phase, progress });
        };
        let client = reqwest::Client::new();
        let mut response = client
            .get(&model.download_url)
            .send()
            .await
            .map_err(|e| format!("Download request failed: {}", e))?;

        let total_size = response.content_length().unwrap_or(model.file_size);
        let mut downloaded: u64 = 0;
        let mut last_progress: f64 = -1.0;

        match model.engine.as_str() {
            "Parakeet" => {
                let archive_path = self.models_dir.join(format!("{}.tar.bz2", model.size));

                {
                    let mut file = tokio::fs::File::create(&archive_path)
                        .await
                        .map_err(|e| format!("Failed to create archive file: {}", e))?;
                    use tokio::io::AsyncWriteExt;
                    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
                        downloaded += chunk.len() as u64;
                        let pct = (downloaded as f64 / total_size as f64) * 100.0;
                        if pct - last_progress >= 0.5 || pct >= 100.0 {
                            report(DownloadPhase::Downloading, pct);
                            last_progress = pct;
                        }
                    }
                    file.flush().await.map_err(|e| e.to_string())?;
                }

                // Extraction can take a while for large archives; switch the
                // UI to an explicit extracting phase so it doesn't sit at 100%.
                report(DownloadPhase::Extracting, 100.0);

                let target_dir = self
                    .models_dir
                    .join("transcription")
                    .join("parakeet")
                    .join(&model.size);
                crate::archive::extract_archive(
                    &archive_path,
                    &target_dir,
                    crate::archive::ExtractLayout::PreservePaths,
                )
                .map_err(|e| format!("Failed to extract {}: {}", model.size, e))?;

                // Remove the archive after extraction
                let _ = std::fs::remove_file(&archive_path);

                Ok(target_dir)
            }
            _ => {
                let path = self.get_model_path(model);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut file = tokio::fs::File::create(&path)
                    .await
                    .map_err(|e| format!("Failed to create model file: {}", e))?;

                use tokio::io::AsyncWriteExt;
                while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                    file.write_all(&chunk).await.map_err(|e| e.to_string())?;
                    downloaded += chunk.len() as u64;
                    let pct = (downloaded as f64 / total_size as f64) * 100.0;
                    if pct - last_progress >= 0.5 || pct >= 100.0 {
                        report(DownloadPhase::Downloading, pct);
                        last_progress = pct;
                    }
                }

                file.flush().await.map_err(|e| e.to_string())?;
                report(DownloadPhase::Downloading, 100.0);
                Ok(path)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_model_exists() {
        let model = ModelManager::find_model("Whisper.cpp", "tiny.en");
        assert!(model.is_some());
        assert_eq!(model.unwrap().engine, "Whisper.cpp");
    }

    #[test]
    fn find_model_gpu_exists() {
        let model = ModelManager::find_model("Whisper.cpp (GPU)", "tiny.en");
        assert!(model.is_some());
        assert_eq!(model.unwrap().engine, "Whisper.cpp (GPU)");
    }

    #[test]
    fn find_model_parakeet_exists() {
        let model = ModelManager::find_model("Parakeet", "parakeet-tdt-0.6b-v3");
        assert!(model.is_some());
        assert_eq!(model.unwrap().engine, "Parakeet");
    }

    #[test]
    fn find_model_not_found() {
        let model = ModelManager::find_model("Whisper.cpp", "nonexistent-model");
        assert!(model.is_none());
    }

    #[test]
    fn find_model_wrong_engine() {
        let model = ModelManager::find_model("Parakeet", "tiny.en");
        assert!(model.is_none());
    }

    #[test]
    fn get_available_engines_includes_all() {
        let engines = ModelManager::get_available_engines();
        assert!(engines.contains(&"Whisper.cpp".to_string()));
        assert!(engines.contains(&"Whisper.cpp (GPU)".to_string()));
        assert!(engines.contains(&"Parakeet".to_string()));
        assert!(!engines.contains(&"Post-Process (Local)".to_string()));
    }

    #[test]
    fn get_available_engines_no_duplicates() {
        let engines = ModelManager::get_available_engines();
        let mut sorted = engines.clone();
        sorted.dedup();
        assert_eq!(engines.len(), sorted.len());
    }

    #[test]
    fn get_model_path_whisper_cpp() {
        let manager = ModelManager::new().unwrap();
        let model = ModelManager::find_model("Whisper.cpp", "tiny.en").unwrap();
        let path = manager.get_model_path(&model);
        assert!(path.ends_with(
            std::path::Path::new("transcription")
                .join("whisper")
                .join("ggml-tiny.en.bin")
        ));
    }

    #[test]
    fn get_model_path_parakeet() {
        let manager = ModelManager::new().unwrap();
        let model = ModelManager::find_model("Parakeet", "parakeet-tdt-0.6b-v3").unwrap();
        let path = manager.get_model_path(&model);
        assert!(path.ends_with(
            std::path::Path::new("transcription")
                .join("parakeet")
                .join("parakeet-tdt-0.6b-v3")
        ));
    }

    #[test]
    fn is_model_downloaded_returns_false_for_nonexistent() {
        let manager = ModelManager::new().unwrap();
        let model = ModelManager::find_model("Whisper.cpp", "nonexistent").unwrap_or(ModelInfo {
            engine: "Whisper.cpp".to_string(),
            size: "__test_nonexistent__".to_string(),
            file_size: 0,
            download_url: String::new(),
            sha256: String::new(),
            label: String::new(),
            description: String::new(),
            recommended: false,
            category: "transcription".to_string(),
        });
        assert!(!manager.is_model_downloaded(&model));
    }

    #[test]
    fn model_info_fields_are_populated() {
        let models = ModelManager::get_available_models();
        assert!(!models.is_empty());
        for model in &models {
            assert!(!model.engine.is_empty());
            assert!(!model.size.is_empty());
            assert!(!model.label.is_empty());
            assert!(!model.download_url.is_empty());
            assert!(model.file_size > 0);
        }
    }
}
