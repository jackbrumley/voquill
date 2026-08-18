use crate::config::Config;
use crate::local_whisper::{self, WhisperEngineCache};
use crate::parakeet;
use crate::transcription::{self, TranscriptionError, TranscriptionService};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Single owner of all engine-specific shared state. Provides a factory method
/// that returns the right `TranscriptionService` for the current config, so
/// callers (`recording_flow`, `bootstrap`) never need to know about individual
/// engine types or their caches.
pub struct EngineFactory {
    whisper_cache: WhisperEngineCache,
    whisper_last_gpu_error: Arc<Mutex<Option<String>>>,
    gpu_tested: AtomicBool,
    /// Serializes preload attempts so fire-and-forget warm-ups (startup,
    /// config save, download completion) never double-load a model alongside
    /// an awaited preload requested by the frontend.
    preload_lock: Arc<tokio::sync::Mutex<()>>,
}

/// Returns true if the engine name indicates GPU acceleration should be used.
/// Single owner of the "(GPU)" naming convention across all engines
/// (transcription and post-processing).
pub(crate) fn engine_uses_gpu(engine_name: &str) -> bool {
    engine_name.contains("(GPU)")
}

/// Describes what settings an engine supports, so the frontend can render the
/// appropriate configuration UI.
#[derive(Serialize)]
pub struct EngineCapabilities {
    pub gpu_supported: bool,
    pub settings: Vec<EngineSetting>,
}

#[derive(Serialize)]
pub struct EngineSetting {
    pub key: String,
    pub label: String,
    pub description: String,
    #[serde(rename = "settingType")]
    pub setting_type: String,
    pub default: serde_json::Value,
    pub options: Option<Vec<SettingOption>>,
}

#[derive(Serialize)]
pub struct SettingOption {
    pub value: String,
    pub label: String,
}

impl Default for EngineFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineFactory {
    pub fn new() -> Self {
        Self {
            whisper_cache: Arc::new(Mutex::new(None)),
            whisper_last_gpu_error: Arc::new(Mutex::new(None)),
            gpu_tested: AtomicBool::new(false),
            preload_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// Whether a GPU load attempt has been made since startup.
    pub fn gpu_has_been_tested(&self) -> bool {
        self.gpu_tested.load(Ordering::SeqCst)
    }

    /// The most recent GPU error, if any. Returns `None` if the last GPU
    /// attempt succeeded, or if no GPU attempt has been made yet.
    pub fn last_gpu_error(&self) -> Option<String> {
        self.whisper_last_gpu_error.lock().unwrap().clone()
    }

    /// Returns the capabilities and settings for a given engine name, so the
    /// frontend can render per-engine configuration controls.
    pub fn engine_capabilities(engine_name: &str) -> EngineCapabilities {
        match engine_name {
            "Whisper.cpp" => EngineCapabilities {
                gpu_supported: false,
                settings: vec![EngineSetting {
                    key: "whisper.num_threads".to_string(),
                    label: "Thread Count".to_string(),
                    description: "CPU threads for whisper.cpp inference. More threads = faster, but uses more CPU.".to_string(),
                    setting_type: "number".to_string(),
                    default: serde_json::json!(4),
                    options: None,
                }],
            },
            "Whisper.cpp (GPU)" => EngineCapabilities {
                gpu_supported: true,
                settings: vec![],
            },
            "Parakeet" => EngineCapabilities {
                gpu_supported: false,
                settings: vec![EngineSetting {
                    key: "parakeet.num_threads".to_string(),
                    label: "Thread Count".to_string(),
                    description: "CPU threads for sherpa-onnx inference. More threads = faster, but uses more CPU.".to_string(),
                    setting_type: "number".to_string(),
                    default: serde_json::json!(2),
                    options: None,
                }],
            },
            _ => EngineCapabilities {
                gpu_supported: false,
                settings: vec![],
            },
        }
    }

    pub async fn create_service(
        &self,
        config: &Config,
    ) -> Result<Box<dyn TranscriptionService + Send + Sync>, TranscriptionError> {
        match config.transcription_mode {
            crate::config::TranscriptionMode::Api => {
                Ok(Box::new(transcription::APITranscriptionService {
                    api_key: config.openai_api_key.clone(),
                    api_url: config.api_url.clone(),
                    api_model: config.api_model.clone(),
                }))
            }
            crate::config::TranscriptionMode::Local => {
                let use_gpu = engine_uses_gpu(&config.local_engine);
                if use_gpu {
                    self.gpu_tested.store(true, Ordering::SeqCst);
                }
                match config.local_engine.as_str() {
                    "Whisper.cpp" | "Whisper.cpp (GPU)" => {
                        let service = local_whisper::LocalWhisperService::new_full(
                            self.whisper_cache.clone(),
                            &config.local_model_size,
                            use_gpu,
                            Some(self.whisper_last_gpu_error.clone()),
                        )?;
                        Ok(Box::new(service))
                    }
                    "Parakeet" => {
                        let model_info =
                            crate::model_manager::ModelManager::find_model(
                                "Parakeet",
                                &config.local_model_size,
                            )
                            .ok_or_else(|| {
                                TranscriptionError::Model(format!(
                                    "Model {} not found for Parakeet engine",
                                    config.local_model_size
                                ))
                            })?;
                        let model_dir = crate::model_manager::ModelManager::new()
                            .map_err(TranscriptionError::Model)?
                            .get_model_path(&model_info);
                        if !model_dir.exists() {
                            return Err(TranscriptionError::Model(format!(
                                "Model {} is not downloaded. Please download it in settings.",
                                config.local_model_size
                            )));
                        }
                        let service =
                            parakeet::ParakeetService::new(model_dir, &config.local_model_size)
                                .await?;
                        Ok(Box::new(service))
                    }
                    other => Err(TranscriptionError::Model(format!(
                        "Unknown local engine: {}. Available engines: Whisper.cpp, Whisper.cpp (GPU), Parakeet",
                        other
                    ))),
                }
            }
        }
    }

    pub async fn preload(&self, config: &Config) {
        let _preload_guard = self.preload_lock.lock().await;

        if config.transcription_mode != crate::config::TranscriptionMode::Local {
            return;
        }

        let model_info = match crate::model_manager::ModelManager::find_model(
            &config.local_engine,
            &config.local_model_size,
        ) {
            Some(m) => m,
            None => {
                crate::log_info!(
                    "Engine preload: model {} not found for engine {}; skipping",
                    config.local_model_size,
                    config.local_engine
                );
                return;
            }
        };

        let model_path = match crate::model_manager::ModelManager::new()
            .map(|m| m.get_model_path(&model_info))
        {
            Ok(p) if p.exists() => p,
            _ => {
                crate::log_info!(
                    "Engine preload: model {} not downloaded yet; skipping",
                    config.local_model_size
                );
                return;
            }
        };

        let use_gpu = engine_uses_gpu(&config.local_engine);
        if use_gpu {
            self.gpu_tested.store(true, Ordering::SeqCst);
        }

        crate::log_info!(
            "Engine preload: starting (engine={}, model={}, gpu={})",
            config.local_engine,
            config.local_model_size,
            use_gpu
        );

        let result = local_whisper::ensure_model_loaded_with_fallback(
            &self.whisper_cache,
            model_path,
            config.local_model_size.clone(),
            use_gpu,
            None,
        )
        .await;

        match result {
            Ok(outcome) => {
                if let Some(ref reason) = outcome.fell_back_from_gpu {
                    crate::log_warn!(
                        "Engine preload: model {} loaded on CPU; GPU error: {}",
                        config.local_model_size,
                        reason
                    );
                    *self.whisper_last_gpu_error.lock().unwrap() = Some(reason.clone());
                } else {
                    crate::log_info!(
                        "Engine preload: model {} loaded (gpu={})",
                        config.local_model_size,
                        outcome.use_gpu
                    );
                }
            }
            Err(e) => {
                crate::log_warn!(
                    "Engine preload: failed to load {} ({}): {}",
                    config.local_engine,
                    config.local_model_size,
                    e
                );
            }
        }
    }

    pub fn unload_all(&self) {
        local_whisper::unload_model(&self.whisper_cache);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_uses_gpu_true_for_gpu_engine() {
        assert!(engine_uses_gpu("Whisper.cpp (GPU)"));
    }

    #[test]
    fn engine_uses_gpu_false_for_cpu_engine() {
        assert!(!engine_uses_gpu("Whisper.cpp"));
        assert!(!engine_uses_gpu("Parakeet"));
    }

    #[test]
    fn engine_capabilities_whisper_cpp() {
        let caps = EngineFactory::engine_capabilities("Whisper.cpp");
        assert!(!caps.gpu_supported);
        assert!(!caps.settings.is_empty());
        assert!(caps.settings.iter().any(|s| s.key == "whisper.num_threads"));
    }

    #[test]
    fn engine_capabilities_whisper_cpp_gpu() {
        let caps = EngineFactory::engine_capabilities("Whisper.cpp (GPU)");
        assert!(caps.gpu_supported);
        assert!(caps.settings.is_empty());
    }

    #[test]
    fn engine_capabilities_parakeet() {
        let caps = EngineFactory::engine_capabilities("Parakeet");
        assert!(!caps.gpu_supported);
        assert!(!caps.settings.is_empty());
        assert!(caps
            .settings
            .iter()
            .any(|s| s.key == "parakeet.num_threads"));
    }

    #[test]
    fn engine_capabilities_unknown_engine() {
        let caps = EngineFactory::engine_capabilities("UnknownEngine");
        assert!(!caps.gpu_supported);
        assert!(caps.settings.is_empty());
    }

    #[test]
    fn engine_capabilities_setting_types() {
        let caps = EngineFactory::engine_capabilities("Whisper.cpp");
        for setting in &caps.settings {
            assert!(!setting.key.is_empty());
            assert!(!setting.label.is_empty());
            assert!(!setting.description.is_empty());
            assert!(["number", "bool", "select"].contains(&setting.setting_type.as_str()));
        }
    }
}
