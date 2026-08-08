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

    pub fn get_available_models() -> Vec<ModelInfo> {
        let cpu_models = Self::cpu_models();
        let gpu_models = Self::gpu_models();
        let mut all = Vec::with_capacity(cpu_models.len() + gpu_models.len());
        all.extend(cpu_models);
        all.extend(gpu_models);
        all
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

    pub fn get_model_path(&self, model_size: &str) -> PathBuf {
        self.models_dir.join(format!("ggml-{}.bin", model_size))
    }

    pub fn is_model_downloaded(&self, model_size: &str) -> bool {
        self.get_model_path(model_size).exists()
    }

    pub async fn download_model<F>(
        &self,
        model_size: &str,
        progress_callback: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(f64) + Send + 'static,
    {
        let models = Self::get_available_models();
        let model_info = models
            .iter()
            .find(|m| m.size == model_size)
            .ok_or_else(|| format!("Model size {} not found", model_size))?;

        let path = self.get_model_path(model_size);

        let client = reqwest::Client::new();
        let mut response = client
            .get(&model_info.download_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let total_size = response.content_length().unwrap_or(model_info.file_size);
        let mut downloaded: u64 = 0;
        let mut last_reported_progress: f64 = -1.0;

        let mut file = tokio::fs::File::create(&path)
            .await
            .map_err(|e| e.to_string())?;

        use tokio::io::AsyncWriteExt;
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;

            let progress = (downloaded as f64 / total_size as f64) * 100.0;

            // Only report progress if it has increased by at least 0.5%
            // to prevent saturating the Tauri IPC bridge and freezing the UI
            if progress - last_reported_progress >= 0.5 || progress >= 100.0 {
                progress_callback(progress);
                last_reported_progress = progress;
            }
        }

        file.flush().await.map_err(|e| e.to_string())?;
        Ok(path)
    }
}
