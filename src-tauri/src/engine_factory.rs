use crate::config::Config;
use crate::engine_residency::{EngineResidency, ResidencyTracker};
use crate::engines::{EngineFamily, ModelEngine};
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
    gpu_last_error: Arc<Mutex<Option<String>>>,
    gpu_tested: AtomicBool,
    /// Serializes preload attempts so fire-and-forget warm-ups (startup,
    /// config save, download completion) never double-load a model alongside
    /// an awaited preload requested by the frontend.
    preload_lock: Arc<tokio::sync::Mutex<()>>,
    /// The warm Parakeet sherpa-onnx sidecar, reused across dictations.
    parakeet_sidecar: parakeet::ParakeetSidecarCache,
    /// Reports whether an engine is in memory and when it was last used.
    residency: ResidencyTracker,
}

/// Marks a model load as running for as long as it is alive, so the UI shows
/// the engine loading and then whatever the caches hold once it finishes.
struct LoadInProgress<'a> {
    factory: &'a EngineFactory,
}

impl<'a> LoadInProgress<'a> {
    fn start(factory: &'a EngineFactory) -> Self {
        factory.residency.begin_load();
        factory.publish_residency();
        Self { factory }
    }
}

impl Drop for LoadInProgress<'_> {
    fn drop(&mut self) {
        self.factory.residency.end_load();
        self.factory.publish_residency();
    }
}

/// Generates dropdown options for CPU thread allocation, placing Auto at the top
/// and populating all hardware cores down to 1.
fn build_thread_options(max_cpus: usize) -> Vec<SettingOption> {
    let mut options = Vec::with_capacity(max_cpus + 1);
    options.push(SettingOption {
        value: "auto".to_string(),
        label: format!("Auto ({} Cores - Recommended)", max_cpus),
    });
    for i in (1..=max_cpus).rev() {
        let label = if i == 1 {
            "1 Core (Minimal CPU)".to_string()
        } else if i == max_cpus {
            format!("{} Cores (All Cores)", i)
        } else {
            format!("{} Cores", i)
        };
        options.push(SettingOption {
            value: i.to_string(),
            label,
        });
    }
    options
}

/// Resolves the configured thread count for an engine, falling back to all
/// available logical CPU cores for maximum speed when unspecified.
fn resolve_thread_count(config: &Config, key: &str) -> usize {
    let max_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    if let Some(ref ec) = config.engine_config {
        if let Some(val) = ec.get(key) {
            if let Some(s) = val.as_str() {
                if s == "auto" || s == "0" {
                    return max_cpus;
                }
                if let Ok(num) = s.parse::<usize>() {
                    if num > 0 {
                        return num.clamp(1, 64);
                    }
                }
            } else if let Some(num) = val.as_u64() {
                if num > 0 {
                    return (num as usize).clamp(1, 64);
                }
            }
        }
    }
    max_cpus
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
            gpu_last_error: Arc::new(Mutex::new(None)),
            gpu_tested: AtomicBool::new(false),
            preload_lock: Arc::new(tokio::sync::Mutex::new(())),
            parakeet_sidecar: parakeet::ParakeetSidecarCache::new(),
            residency: ResidencyTracker::new(),
        }
    }

    /// Connects residency reporting to the app windows.
    pub fn attach_app_handle(&self, app_handle: tauri::AppHandle) {
        self.residency.attach_app_handle(app_handle);
    }

    /// Whether the local speech engine is unloaded, loading or loaded.
    pub fn residency(&self) -> EngineResidency {
        self.residency.resolve(self.engine_in_memory())
    }

    fn engine_in_memory(&self) -> bool {
        self.whisper_cache.lock().unwrap().is_some() || self.parakeet_sidecar.is_loaded()
    }

    fn publish_residency(&self) {
        self.residency.publish(self.residency());
    }

    /// Unloads the engine if it is loaded and has not been used for at least
    /// `idle_limit`. Returns whether it unloaded.
    pub fn unload_if_idle(&self, idle_limit: std::time::Duration) -> bool {
        if self.residency() != EngineResidency::Loaded || self.residency.idle_for() < idle_limit {
            return false;
        }
        self.unload_all();
        true
    }

    /// Whether a GPU load attempt has been made since startup.
    pub fn gpu_has_been_tested(&self) -> bool {
        self.gpu_tested.load(Ordering::SeqCst)
    }

    /// The most recent GPU error, if any. Returns `None` if the last GPU
    /// attempt succeeded, or if no GPU attempt has been made yet.
    pub fn last_gpu_error(&self) -> Option<String> {
        self.gpu_last_error.lock().unwrap().clone()
    }

    /// Returns the capabilities and settings for a given engine name, so the
    /// frontend can render per-engine configuration controls.
    pub fn engine_capabilities(engine_name: &str) -> EngineCapabilities {
        let max_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let cpu_threads_setting = |key: &str| {
            EngineSetting {
            key: key.to_string(),
            label: "CPU Cores".to_string(),
            description: "How many processor cores Voquill uses to turn speech into text. Auto suits most computers.".to_string(),
            setting_type: "select".to_string(),
            default: serde_json::json!("auto"),
            options: Some(build_thread_options(max_cpus)),
        }
        };

        match ModelEngine::parse(engine_name) {
            Some(ModelEngine::WhisperCpu) => EngineCapabilities {
                gpu_supported: false,
                settings: vec![cpu_threads_setting("whisper.num_threads")],
            },
            Some(ModelEngine::ParakeetCpu) => EngineCapabilities {
                gpu_supported: false,
                settings: vec![cpu_threads_setting("parakeet.num_threads")],
            },
            Some(engine) => EngineCapabilities {
                gpu_supported: engine.uses_gpu(),
                settings: vec![],
            },
            None => EngineCapabilities {
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
                self.residency.mark_used();
                // Loading through `preload` reports progress to the UI and
                // waits for a load already running (e.g. the one started when
                // the recording began).
                if self.residency() != EngineResidency::Loaded {
                    self.preload(config).await;
                }
                let engine = config
                    .transcription_engine()
                    .map_err(TranscriptionError::Model)?;
                if engine.uses_gpu() {
                    self.gpu_tested.store(true, Ordering::SeqCst);
                }
                match engine.family() {
                    EngineFamily::Whisper => {
                        let num_threads = resolve_thread_count(config, "whisper.num_threads");
                        let service = local_whisper::LocalWhisperService::new_full(
                            self.whisper_cache.clone(),
                            &config.local_model_size,
                            engine.uses_gpu(),
                            self.gpu_last_error.clone(),
                            num_threads,
                        )?;
                        Ok(Box::new(service))
                    }
                    EngineFamily::Parakeet => Ok(Box::new(self.parakeet_service(config).await?)),
                    EngineFamily::PostProcess => Err(TranscriptionError::Model(format!(
                        "{} is a post-processing engine, not a transcription engine",
                        engine.name()
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

        // Every engine or model change unloads the caches, so a loaded engine
        // is always the configured one.
        if self.residency() == EngineResidency::Loaded {
            return;
        }

        let engine = match config.transcription_engine() {
            Ok(engine) => engine,
            Err(error) => {
                crate::log_warn!("Engine preload: {}", error);
                return;
            }
        };
        let model_info = match crate::model_manager::ModelManager::find_model(
            engine,
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

        let use_gpu = engine.uses_gpu();
        if use_gpu {
            self.gpu_tested.store(true, Ordering::SeqCst);
        }

        let _load = LoadInProgress::start(self);
        crate::log_info!(
            "Engine preload: starting (engine={}, model={}, gpu={})",
            config.local_engine,
            config.local_model_size,
            use_gpu
        );

        match engine.family() {
            EngineFamily::Whisper => {
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
                            *self.gpu_last_error.lock().unwrap() = Some(reason.clone());
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
            EngineFamily::Parakeet => match self.parakeet_service(config).await {
                Ok(_) => crate::log_info!(
                    "Engine preload: Parakeet sidecar running (engine {}, model {})",
                    config.local_engine,
                    config.local_model_size
                ),
                Err(e) => crate::log_warn!(
                    "Engine preload: Parakeet sidecar failed to start ({}): {}",
                    config.local_model_size,
                    e
                ),
            },
            EngineFamily::PostProcess => crate::log_warn!(
                "Engine preload: {} is not a transcription engine",
                engine.name()
            ),
        }
    }

    /// The warm Parakeet sidecar for the configured engine and model:
    /// sherpa-onnx for "Parakeet", parakeet.cpp for "Parakeet (GPU)". A GPU
    /// sidecar that ended up on the CPU is reported through `last_gpu_error`.
    async fn parakeet_service(
        &self,
        config: &Config,
    ) -> Result<parakeet::ParakeetService, TranscriptionError> {
        let engine = config
            .transcription_engine()
            .map_err(TranscriptionError::Model)?;
        let use_gpu = engine.uses_gpu();
        let backend = if use_gpu {
            parakeet::ParakeetBackend::ParakeetCpp
        } else {
            parakeet::ParakeetBackend::SherpaOnnx
        };
        let model_info =
            crate::model_manager::ModelManager::find_model(engine, &config.local_model_size)
                .ok_or_else(|| {
                    TranscriptionError::Model(format!(
                        "Model {} not found for {} engine",
                        config.local_model_size, config.local_engine
                    ))
                })?;
        let manager =
            crate::model_manager::ModelManager::new().map_err(TranscriptionError::Model)?;
        if !manager.is_model_downloaded(&model_info) {
            return Err(TranscriptionError::Model(format!(
                "Model {} is not downloaded. Please download it in settings.",
                config.local_model_size
            )));
        }
        let num_threads = resolve_thread_count(config, "parakeet.num_threads");
        let service = self
            .parakeet_sidecar
            .service(
                backend,
                manager.get_model_path(&model_info),
                &config.local_model_size,
                num_threads,
            )
            .await?;
        if use_gpu {
            let fallback_reason = service.gpu_fallback_reason();
            if let Some(ref reason) = fallback_reason {
                crate::log_warn!("Parakeet (GPU): {}", reason);
            }
            *self.gpu_last_error.lock().unwrap() = fallback_reason;
        }
        Ok(service)
    }

    /// Unloads every cached transcription engine: the Whisper model and the
    /// Parakeet sidecar. Called when the transcription engine, model or
    /// engine settings change (so the old engine never serves the new
    /// configuration and its memory is released), on request from the UI,
    /// and by the idle unload.
    pub fn unload_all(&self) {
        local_whisper::unload_model(&self.whisper_cache);
        self.parakeet_sidecar.invalidate();
        self.publish_residency();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            if setting.setting_type == "select" {
                assert!(setting.options.is_some());
                let options = setting.options.as_ref().unwrap();
                assert!(!options.is_empty());
                assert_eq!(options[0].value, "auto");
            }
        }
    }

    #[test]
    fn thread_options_builds_auto_and_core_list() {
        let options = build_thread_options(8);
        assert_eq!(options.len(), 9);
        assert_eq!(options[0].value, "auto");
        assert_eq!(options[1].value, "8");
        assert_eq!(options[8].value, "1");
    }

    #[test]
    fn resolve_thread_count_handles_auto_and_numbers() {
        let max_cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let mut config = Config {
            engine_config: None,
            ..Default::default()
        };
        assert_eq!(
            resolve_thread_count(&config, "whisper.num_threads"),
            max_cpus
        );

        config.engine_config = Some(serde_json::json!({
            "whisper.num_threads": "auto"
        }));
        assert_eq!(
            resolve_thread_count(&config, "whisper.num_threads"),
            max_cpus
        );

        config.engine_config = Some(serde_json::json!({
            "whisper.num_threads": "4"
        }));
        assert_eq!(resolve_thread_count(&config, "whisper.num_threads"), 4);

        config.engine_config = Some(serde_json::json!({
            "whisper.num_threads": 6
        }));
        assert_eq!(resolve_thread_count(&config, "whisper.num_threads"), 6);
    }
}
