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
}

/// The required model files for a Parakeet / sherpa-onnx model directory.
const PARAKEET_REQUIRED_FILES: &[&str] = &[
    "encoder.int8.onnx",
    "decoder.int8.onnx",
    "joiner.int8.onnx",
    "tokens.txt",
];

pub struct ModelManager {
    pub models_dir: PathBuf,
}

impl ModelManager {
    pub fn new() -> Result<Self, String> {
        let models_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("foss-voquill")
            .join("models");

        if !models_dir.exists() {
            std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
        }

        Ok(Self { models_dir })
    }

    /// Looks up a single model by engine name + model size.
    pub fn find_model(engine: &str, size: &str) -> Option<ModelInfo> {
        Self::get_available_models()
            .into_iter()
            .find(|m| m.engine == engine && m.size == size)
    }

    pub fn get_available_models() -> Vec<ModelInfo> {
        let cpu_models = Self::cpu_models();
        let gpu_models = Self::gpu_models();
        let mut all = Vec::with_capacity(cpu_models.len() + gpu_models.len());
        all.extend(cpu_models);
        all.extend(gpu_models);
        all.extend(Self::parakeet_models());
        all
    }

    /// The on-disk path for a model, taking its engine into account.
    /// whisper.cpp models are flat files:  models/ggml-{size}.bin
    /// Parakeet models are directories:    models/parakeet/{size}/
    pub fn get_model_path(&self, model: &ModelInfo) -> PathBuf {
        match model.engine.as_str() {
            e if e.contains("Whisper.cpp") => {
                self.models_dir.join(format!("ggml-{}.bin", model.size))
            }
            _ => self.models_dir.join("parakeet").join(&model.size),
        }
    }

    /// Checks whether the model is present on disk. For whisper.cpp this is a
    /// single file check. For Parakeet it checks for all required ONNX files.
    pub fn is_model_downloaded(&self, model: &ModelInfo) -> bool {
        match model.engine.as_str() {
            e if e.starts_with("Parakeet") => {
                let dir = self.models_dir.join("parakeet").join(&model.size);
                PARAKEET_REQUIRED_FILES.iter().all(|f| dir.join(f).exists())
            }
            _ => {
                let path = self.models_dir.join(format!("ggml-{}.bin", model.size));
                path.exists()
            }
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
        F: Fn(f64) + Send + 'static,
    {
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
            e if e.starts_with("Parakeet") => {
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
                            progress_callback(pct);
                            last_progress = pct;
                        }
                    }
                    file.flush().await.map_err(|e| e.to_string())?;
                }

                // Extract tar.bz2 archive
                let target_dir = self.models_dir.join("parakeet").join(&model.size);
                std::fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;

                let archive_file = std::fs::File::open(&archive_path)
                    .map_err(|e| format!("Failed to open archive: {}", e))?;
                let decoder = bzip2::read::BzDecoder::new(archive_file);
                let mut archive = tar::Archive::new(decoder);
                archive
                    .unpack(&target_dir)
                    .map_err(|e| format!("Failed to extract model archive: {}", e))?;

                // Remove the archive after extraction
                let _ = std::fs::remove_file(&archive_path);

                progress_callback(100.0);
                Ok(target_dir)
            }
            _ => {
                let path = self.get_model_path(model);
                let mut file = tokio::fs::File::create(&path)
                    .await
                    .map_err(|e| format!("Failed to create model file: {}", e))?;

                use tokio::io::AsyncWriteExt;
                while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                    file.write_all(&chunk).await.map_err(|e| e.to_string())?;
                    downloaded += chunk.len() as u64;
                    let pct = (downloaded as f64 / total_size as f64) * 100.0;
                    if pct - last_progress >= 0.5 || pct >= 100.0 {
                        progress_callback(pct);
                        last_progress = pct;
                    }
                }

                file.flush().await.map_err(|e| e.to_string())?;
                progress_callback(100.0);
                Ok(path)
            }
        }
    }

    fn cpu_models() -> Vec<ModelInfo> {
        vec![
            Self::model_info("Whisper.cpp", "tiny.en", "Tiny (English)", 77_600_000,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
                "be07098a4cc50130a511ca096303ad371c513297a7d4a093047d9ca4378f8776",
                "Lightning fast, best for simple commands.", false),
            Self::model_info("Whisper.cpp", "distil-small.en", "Distil-Small (English)", 175_000_000,
                "https://huggingface.co/distil-whisper/distil-small.en/resolve/main/ggml-distil-small.en.bin",
                "e8a676964fd3f78b021a385f078a18863712ca10fdc907a685eee9c0e71d7a62",
                "Perfect balance of speed and high accuracy.", true),
            Self::model_info("Whisper.cpp", "base.en", "Base (English)", 147_000_000,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
                "60ed30914c83ad34005b63359d992f802773d57864f7df26e95261895697d74d",
                "Standard choice for general dictation.", false),
            Self::model_info("Whisper.cpp", "small.en", "Small (English)", 483_000_000,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
                "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1",
                "Great accuracy for complex vocabulary.", false),
            Self::model_info("Whisper.cpp", "medium.en", "Medium (English)", 1_500_000_000,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
                "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1",
                "Highest accuracy. Needs a powerful computer or GPU.", false),
        ]
    }

    fn gpu_models() -> Vec<ModelInfo> {
        vec![
            Self::model_info("Whisper.cpp (GPU)", "tiny.en", "Tiny (English)", 77_600_000,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
                "be07098a4cc50130a511ca096303ad371c513297a7d4a093047d9ca4378f8776",
                "Lightning fast with GPU acceleration. Requires a compatible GPU.", false),
            Self::model_info("Whisper.cpp (GPU)", "distil-small.en", "Distil-Small (English)", 175_000_000,
                "https://huggingface.co/distil-whisper/distil-small.en/resolve/main/ggml-distil-small.en.bin",
                "e8a676964fd3f78b021a385f078a18863712ca10fdc907a685eee9c0e71d7a62",
                "Fast and accurate with GPU acceleration. Requires a compatible GPU.", true),
            Self::model_info("Whisper.cpp (GPU)", "base.en", "Base (English)", 147_000_000,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
                "60ed30914c83ad34005b63359d992f802773d57864f7df26e95261895697d74d",
                "Standard choice with GPU acceleration. Requires a compatible GPU.", false),
            Self::model_info("Whisper.cpp (GPU)", "small.en", "Small (English)", 483_000_000,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
                "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1",
                "Great accuracy with GPU acceleration. Requires a compatible GPU.", false),
            Self::model_info("Whisper.cpp (GPU)", "medium.en", "Medium (English)", 1_500_000_000,
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
                "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1",
                "Highest accuracy with GPU acceleration. Requires a compatible GPU.", false),
        ]
    }

    fn parakeet_models() -> Vec<ModelInfo> {
        vec![
            Self::model_info("Parakeet", "parakeet-tdt-0.6b-v3", "Parakeet TDT 0.6B (Multilingual)", 680_000_000,
                "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2",
                "",
                "NVIDIA Parakeet model, 25 languages. Requires sherpa-onnx sidecar. Fast on CPU.", true),
            Self::model_info("Parakeet", "parakeet-unified-en-0.6b", "Parakeet Unified EN 0.6B (English)", 631_000_000,
                "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-unified-en-0.6b-int8-non-streaming.tar.bz2",
                "",
                "NVIDIA Parakeet English-only model. Requires sherpa-onnx sidecar. Fast on CPU.", false),
        ]
    }

    #[allow(clippy::too_many_arguments)]
    fn model_info(
        engine: &str,
        size: &str,
        label: &str,
        file_size: u64,
        download_url: &str,
        sha256: &str,
        description: &str,
        recommended: bool,
    ) -> ModelInfo {
        ModelInfo {
            engine: engine.to_string(),
            size: size.to_string(),
            label: label.to_string(),
            file_size,
            download_url: download_url.to_string(),
            sha256: sha256.to_string(),
            description: description.to_string(),
            recommended,
        }
    }

    pub fn get_available_engines() -> Vec<String> {
        let mut engines: Vec<String> = Self::get_available_models()
            .iter()
            .map(|m| m.engine.clone())
            .collect();
        engines.sort();
        engines.dedup();
        engines
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
        assert!(path.to_string_lossy().ends_with("ggml-tiny.en.bin"));
    }

    #[test]
    fn get_model_path_parakeet() {
        let manager = ModelManager::new().unwrap();
        let model = ModelManager::find_model("Parakeet", "parakeet-tdt-0.6b-v3").unwrap();
        let path = manager.get_model_path(&model);
        assert!(path
            .to_string_lossy()
            .ends_with("parakeet/parakeet-tdt-0.6b-v3"));
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
