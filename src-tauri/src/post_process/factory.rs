use crate::config::Config;
use crate::post_process::provider_api::APIPostProcessService;
use crate::post_process::provider_local::SidecarPostProcess;
use crate::post_process::PostProcessService;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Fingerprint of the configuration a cached local sidecar was started with.
/// Any change tears the sidecar down and starts a fresh one.
#[derive(PartialEq)]
struct LocalFingerprint {
    engine: String,
    model: String,
    system_prompt: String,
}

/// The cached local sidecar service paired with its configuration fingerprint.
type CachedLocalService = Arc<Mutex<Option<(LocalFingerprint, Arc<SidecarPostProcess>)>>>;

/// Single owner of post-process service lifecycle. Local sidecar services are
/// expensive to start (process spawn + GGUF model load), so the most recent
/// one is cached and reused across dictations, mirroring `EngineFactory`'s
/// model cache for transcription.
pub struct PostProcessFactory {
    cached_local: CachedLocalService,
    gpu_tested: AtomicBool,
    last_gpu_error: Arc<Mutex<Option<String>>>,
}

impl PostProcessFactory {
    pub fn new() -> Self {
        Self {
            cached_local: Arc::new(Mutex::new(None)),
            gpu_tested: AtomicBool::new(false),
            last_gpu_error: Arc::new(Mutex::new(None)),
        }
    }

    /// Whether a GPU start has been attempted since startup.
    pub fn gpu_has_been_tested(&self) -> bool {
        self.gpu_tested.load(Ordering::SeqCst)
    }

    /// The most recent GPU start error, if any. `None` if the last GPU
    /// attempt succeeded, or if no GPU attempt has been made yet.
    pub fn last_gpu_error(&self) -> Option<String> {
        self.last_gpu_error.lock().unwrap().clone()
    }

    /// Returns the post-process service for the current config. Local
    /// services come from the cache when the configuration is unchanged; API
    /// services are cheap and stateless, so they are constructed per call.
    pub async fn get_service(
        &self,
        config: &Config,
    ) -> Result<Arc<dyn PostProcessService + Send + Sync>, String> {
        match config.post_process_provider {
            crate::config::PostProcessProvider::Api => Ok(Arc::new(APIPostProcessService {
                api_key: config.post_process_api_key.clone(),
                api_url: config.post_process_api_url.clone(),
                model: config.post_process_api_model.clone(),
                system_prompt: config.post_process_prompt.clone(),
            })),
            crate::config::PostProcessProvider::Local => {
                let fingerprint = LocalFingerprint {
                    engine: config.post_process_engine.clone(),
                    model: config.post_process_model.clone(),
                    system_prompt: config.post_process_prompt.clone(),
                };

                {
                    let guard = self.cached_local.lock().unwrap();
                    if let Some((cached_fingerprint, service)) = guard.as_ref() {
                        if *cached_fingerprint == fingerprint {
                            crate::log_info!("Reusing warm llama-server post-process service");
                            return Ok(service.clone());
                        }
                    }
                }

                if crate::engine_factory::engine_uses_gpu(&config.post_process_engine) {
                    self.gpu_tested.store(true, Ordering::SeqCst);
                }

                // Build outside the lock: spawning the sidecar takes seconds.
                // Dictation sessions are serialized by SessionState, so a
                // duplicate concurrent build is not a practical concern.
                let service = Arc::new(
                    SidecarPostProcess::new(
                        &config.post_process_engine,
                        &config.post_process_model,
                        &config.post_process_prompt,
                        self.last_gpu_error.clone(),
                    )
                    .await
                    .map_err(|e| format!("Failed to start local post-process: {}", e))?,
                );

                let mut guard = self.cached_local.lock().unwrap();
                *guard = Some((fingerprint, service.clone()));
                Ok(service)
            }
        }
    }

    /// Drops the cached local sidecar, killing the llama-server process, so
    /// the next request starts a fresh one. Used after failures and on
    /// factory reset.
    pub fn invalidate_local(&self) {
        let mut guard = self.cached_local.lock().unwrap();
        if guard.take().is_some() {
            crate::log_info!("Post-process sidecar cache invalidated");
        }
    }

    /// Warms the local sidecar at startup when post-processing is enabled, so
    /// the first dictation doesn't pay the spawn + model load cost.
    pub async fn preload(&self, config: &Config) {
        if !config.post_process_enabled {
            return;
        }
        if config.post_process_provider != crate::config::PostProcessProvider::Local {
            return;
        }
        crate::log_info!("Post-process preload: starting llama-server warm-up");
        match self.get_service(config).await {
            Ok(_) => crate::log_info!("Post-process preload: llama-server warm"),
            Err(e) => crate::log_warn!("Post-process preload failed: {}", e),
        }
    }
}
