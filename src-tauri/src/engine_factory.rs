use crate::config::Config;
use crate::local_whisper::{self, WhisperEngineCache};
use crate::transcription::{self, TranscriptionError, TranscriptionService};
use std::sync::{Arc, Mutex};

/// Single owner of all engine-specific shared state. Provides a factory method
/// that returns the right `TranscriptionService` for the current config, so
/// callers (`recording_flow`, `bootstrap`) never need to know about individual
/// engine types or their caches.
pub struct EngineFactory {
    whisper_cache: WhisperEngineCache,
    whisper_last_gpu_error: Arc<Mutex<Option<String>>>,
}

impl EngineFactory {
    pub fn new() -> Self {
        Self {
            whisper_cache: Arc::new(Mutex::new(None)),
            whisper_last_gpu_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn create_service(
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
            crate::config::TranscriptionMode::Local => match config.local_engine.as_str() {
                "Whisper.cpp" => {
                    let service = local_whisper::LocalWhisperService::new_full(
                        self.whisper_cache.clone(),
                        &config.local_model_size,
                        config.enable_gpu,
                        Some(self.whisper_last_gpu_error.clone()),
                    )?;
                    Ok(Box::new(service))
                }
                other => Err(TranscriptionError::Model(format!(
                    "Unknown local engine: {}. Available engines: Whisper.cpp",
                    other
                ))),
            },
        }
    }

    pub async fn preload(&self, config: &Config) {
        if config.transcription_mode != crate::config::TranscriptionMode::Local {
            return;
        }

        let model_path = match crate::model_manager::ModelManager::new()
            .map(|m| m.get_model_path(&config.local_model_size))
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

        crate::log_info!(
            "Engine preload: starting (engine={}, model={}, gpu={})",
            config.local_engine,
            config.local_model_size,
            config.enable_gpu
        );

        let result = local_whisper::ensure_model_loaded_with_fallback(
            &self.whisper_cache,
            model_path,
            config.local_model_size.clone(),
            config.enable_gpu,
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
