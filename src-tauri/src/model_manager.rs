use std::path::PathBuf;
use serde::Serialize;
use dirs;

#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
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
            .join("voquill")
            .join("models");
        
        if !models_dir.exists() {
            std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;
        }
        
        Ok(Self { models_dir })
    }
    
    pub fn get_available_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                size: "tiny.en".to_string(),
                label: "Tiny (English)".to_string(),
                file_size: 77_600_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin".to_string(),
                sha256: "be07098a4cc50130a511ca096303ad371c513297a7d4a093047d9ca4378f8776".to_string(), // Placeholder, will update if needed
                description: "Lightning fast, best for simple commands.".to_string(),
                recommended: false,
            },
            ModelInfo {
                size: "distil-small.en".to_string(),
                label: "Distil-Small (English)".to_string(),
                file_size: 175_000_000,
                download_url: "https://huggingface.co/distil-whisper/distil-small.en-ggml/resolve/main/ggml-distil-small.en.bin".to_string(),
                sha256: "60ed30914c83ad34005b63359d992f802773d57864f7df26e95261895697d74d".to_string(),
                description: "Perfect balance of speed and high accuracy.".to_string(),
                recommended: true,
            },
            ModelInfo {
                size: "base.en".to_string(),
                label: "Base (English)".to_string(),
                file_size: 147_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin".to_string(),
                sha256: "60ed30914c83ad34005b63359d992f802773d57864f7df26e95261895697d74d".to_string(),
                description: "Standard choice for general dictation.".to_string(),
                recommended: false,
            },
            ModelInfo {
                size: "small.en".to_string(),
                label: "Small (English)".to_string(),
                file_size: 483_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin".to_string(),
                sha256: "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1".to_string(),
                description: "Great accuracy for complex vocabulary.".to_string(),
                recommended: false,
            },
            ModelInfo {
                size: "medium.en".to_string(),
                label: "Medium (English)".to_string(),
                file_size: 1_500_000_000,
                download_url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin".to_string(),
                sha256: "1be3a305f560a8cc0937f268b7ca67270b240561570d55e09d949cf94edb54d1".to_string(),
                description: "Highest accuracy. Needs a powerful computer or GPU.".to_string(),
                recommended: false,
            },
        ]
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
        F: Fn(f64) + Send + 'static
    {
        let models = Self::get_available_models();
        let model_info = models.iter()
            .find(|m| m.size == model_size)
            .ok_or_else(|| format!("Model size {} not found", model_size))?;
        
        let path = self.get_model_path(model_size);
        
        let client = reqwest::Client::new();
        let mut response = client.get(&model_info.download_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let total_size = response.content_length().unwrap_or(model_info.file_size);
        let mut downloaded: u64 = 0;
        let mut last_reported_progress: f64 = -1.0;
        
        let mut file = tokio::fs::File::create(&path).await.map_err(|e| e.to_string())?;
        
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
