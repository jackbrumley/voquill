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

/// One whisper.cpp model definition. Each spec is expanded into a CPU and a
/// GPU engine variant so the two catalogs can never drift apart.
struct WhisperModelSpec {
    size: &'static str,
    label: &'static str,
    file_size: u64,
    download_url: &'static str,
    sha256: &'static str,
    cpu_description: &'static str,
    gpu_description: &'static str,
    recommended: bool,
}

/// One post-process (GGUF) model definition. Same deal: expanded into CPU
/// ("Post-Process (Local)") and GPU ("Post-Process (GPU)") engine variants
/// sharing the same model file.
struct PostProcessModelSpec {
    size: &'static str,
    label: &'static str,
    file_size: u64,
    download_url: &'static str,
    cpu_description: &'static str,
    gpu_description: &'static str,
    recommended: bool,
}

pub struct ModelManager {
    pub models_dir: PathBuf,
}

impl ModelManager {
    pub fn new() -> Result<Self, String> {
        let models_dir = crate::paths::models_dir()?;
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
        all.extend(Self::post_process_models());
        all
    }

    /// The on-disk path for a model, taking its engine into account.
    /// whisper.cpp models are flat files:  models/ggml-{size}.bin
    /// Parakeet models are directories:    models/parakeet/{size}/
    pub fn get_model_path(&self, model: &ModelInfo) -> PathBuf {
        match model.engine.as_str() {
            e if e.contains("Whisper.cpp") => self
                .models_dir
                .join("transcription")
                .join(format!("ggml-{}.bin", model.size)),
            e if e.starts_with("Post-Process") => self
                .models_dir
                .join("post-process")
                .join(format!("{}.gguf", model.size)),
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
                            report(DownloadPhase::Downloading, pct);
                            last_progress = pct;
                        }
                    }
                    file.flush().await.map_err(|e| e.to_string())?;
                }

                // Extraction can take a while for large archives; switch the
                // UI to an explicit extracting phase so it doesn't sit at 100%.
                report(DownloadPhase::Extracting, 100.0);

                let target_dir = self.models_dir.join("parakeet").join(&model.size);
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

    fn whisper_model_specs() -> Vec<WhisperModelSpec> {
        vec![
            WhisperModelSpec {
                size: "tiny.en", label: "Tiny (English)", file_size: 77_600_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
                sha256: "be07098a4cc50130a511ca096303ad371c513297a7d4a093047d9ca4378f8776",
                cpu_description: "Lightning fast, best for simple commands.",
                gpu_description: "Lightning fast with GPU acceleration. Requires a compatible GPU.",
                recommended: false,
            },
            WhisperModelSpec {
                size: "distil-small.en", label: "Distil-Small (English)", file_size: 175_000_000,
                download_url: "https://huggingface.co/distil-whisper/distil-small.en/resolve/main/ggml-distil-small.en.bin",
                sha256: "e8a676964fd3f78b021a385f078a18863712ca10fdc907a685eee9c0e71d7a62",
                cpu_description: "Perfect balance of speed and high accuracy.",
                gpu_description: "Fast and accurate with GPU acceleration. Requires a compatible GPU.",
                recommended: true,
            },
            WhisperModelSpec {
                size: "base.en", label: "Base (English)", file_size: 147_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
                sha256: "60ed30914c83ad34005b63359d992f802773d57864f7df26e95261895697d74d",
                cpu_description: "Standard choice for general dictation.",
                gpu_description: "Standard choice with GPU acceleration. Requires a compatible GPU.",
                recommended: false,
            },
            WhisperModelSpec {
                size: "small.en", label: "Small (English)", file_size: 483_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
                sha256: "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1",
                cpu_description: "Great accuracy for complex vocabulary.",
                gpu_description: "Great accuracy with GPU acceleration. Requires a compatible GPU.",
                recommended: false,
            },
            WhisperModelSpec {
                size: "medium.en", label: "Medium (English)", file_size: 1_500_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
                sha256: "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1",
                cpu_description: "Highest accuracy. Needs a powerful computer or GPU.",
                gpu_description: "Highest accuracy with GPU acceleration. Requires a compatible GPU.",
                recommended: false,
            },
        ]
    }

    fn whisper_models(engine: &'static str, use_gpu: bool) -> Vec<ModelInfo> {
        Self::whisper_model_specs()
            .into_iter()
            .map(|spec| {
                Self::model_info(
                    engine,
                    spec.size,
                    spec.label,
                    spec.file_size,
                    spec.download_url,
                    spec.sha256,
                    if use_gpu {
                        spec.gpu_description
                    } else {
                        spec.cpu_description
                    },
                    spec.recommended,
                    "transcription",
                )
            })
            .collect()
    }

    fn cpu_models() -> Vec<ModelInfo> {
        Self::whisper_models("Whisper.cpp", false)
    }

    fn gpu_models() -> Vec<ModelInfo> {
        Self::whisper_models("Whisper.cpp (GPU)", true)
    }

    fn parakeet_models() -> Vec<ModelInfo> {
        vec![
            Self::model_info("Parakeet", "parakeet-tdt-0.6b-v3", "Parakeet TDT 0.6B (Multilingual)", 680_000_000,
                "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8.tar.bz2",
                "",
                "NVIDIA Parakeet model, 25 languages. Requires sherpa-onnx sidecar. Fast on CPU.", true, "transcription"),
            Self::model_info("Parakeet", "parakeet-unified-en-0.6b", "Parakeet Unified EN 0.6B (English)", 631_000_000,
                "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-unified-en-0.6b-int8-non-streaming.tar.bz2",
                "",
                "NVIDIA Parakeet English-only model. Requires sherpa-onnx sidecar. Fast on CPU.", false, "transcription"),
        ]
    }

    fn post_process_model_specs() -> Vec<PostProcessModelSpec> {
        vec![
            PostProcessModelSpec {
                size: "qwen2.5-1.5b-instruct", label: "Qwen 2.5 1.5B", file_size: 700_000_000,
                download_url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
                cpu_description: "Small local model for post-processing. Fixes punctuation, capitalization, and removes filler words. ~3-5s on CPU.",
                gpu_description: "Small local model for post-processing, GPU-accelerated via Vulkan. Fixes punctuation, capitalization, and removes filler words.",
                recommended: true,
            },
            PostProcessModelSpec {
                size: "llama-3.2-1b-instruct", label: "Llama 3.2 1B", file_size: 650_000_000,
                download_url: "https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf",
                cpu_description: "Meta's lightweight instruct model. Good for post-processing on modest hardware.",
                gpu_description: "Meta's lightweight instruct model with GPU acceleration via Vulkan.",
                recommended: false,
            },
        ]
    }

    fn post_process_models() -> Vec<ModelInfo> {
        ["Post-Process (Local)", "Post-Process (GPU)"]
            .into_iter()
            .flat_map(|engine| {
                let use_gpu = crate::engine_factory::engine_uses_gpu(engine);
                Self::post_process_model_specs()
                    .into_iter()
                    .map(move |spec| {
                        Self::model_info(
                            engine,
                            spec.size,
                            spec.label,
                            spec.file_size,
                            spec.download_url,
                            "",
                            if use_gpu {
                                spec.gpu_description
                            } else {
                                spec.cpu_description
                            },
                            spec.recommended,
                            "post_process",
                        )
                    })
            })
            .collect()
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
        category: &str,
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
            category: category.to_string(),
        }
    }

    pub fn get_available_engines() -> Vec<String> {
        let mut engines: Vec<String> = Self::get_available_models()
            .iter()
            .filter(|m| m.category == "transcription")
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
        assert!(path.to_string_lossy().ends_with("ggml-tiny.en.bin"));
    }

    #[test]
    fn get_model_path_parakeet() {
        let manager = ModelManager::new().unwrap();
        let model = ModelManager::find_model("Parakeet", "parakeet-tdt-0.6b-v3").unwrap();
        let path = manager.get_model_path(&model);
        assert!(path.ends_with(std::path::Path::new("parakeet").join("parakeet-tdt-0.6b-v3")));
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
