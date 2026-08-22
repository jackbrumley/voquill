use crate::config::Config;
use crate::post_process::provider_api::APIPostProcessService;
use crate::post_process::provider_local::SidecarPostProcess;
use crate::post_process::PostProcessService;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Fingerprint of the configuration a cached local sidecar was started with.
/// Any change tears the sidecar down and starts a fresh one. The system
/// prompt is deliberately excluded: it is request-scoped, so editing it must
/// not restart the server.
#[derive(PartialEq)]
struct LocalFingerprint {
    engine: String,
    model: String,
    threads: String,
}

/// The cached local sidecar service paired with its configuration fingerprint.
type CachedLocalService = Arc<Mutex<Option<(LocalFingerprint, Arc<SidecarPostProcess>)>>>;

/// Single owner of post-process service lifecycle. Local sidecar services are
/// expensive to start (process spawn + GGUF model load), so the most recent
/// one is cached and reused across dictations, mirroring `EngineFactory`'s
/// model cache for transcription.
pub struct PostProcessFactory {
    cached_local: CachedLocalService,
    /// Serializes sidecar builds so a warm-up racing a dictation (or a second
    /// warm-up) cannot spawn duplicate llama-server processes.
    build_lock: tokio::sync::Mutex<()>,
    /// Bumped by `invalidate_local`; a build that started before an
    /// invalidation must not populate the cache when it finishes.
    generation: AtomicU64,
    gpu_tested: AtomicBool,
    last_gpu_error: Arc<Mutex<Option<String>>>,
}

impl Default for PostProcessFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl PostProcessFactory {
    pub fn new() -> Self {
        Self {
            cached_local: Arc::new(Mutex::new(None)),
            build_lock: tokio::sync::Mutex::new(()),
            generation: AtomicU64::new(0),
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
            })),
            crate::config::PostProcessProvider::Local => {
                let fingerprint = LocalFingerprint {
                    engine: config.post_process_engine.clone(),
                    model: config.post_process_model.clone(),
                    threads: config.post_process_threads.clone(),
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

                // Serialize builds: a warm-up racing a dictation (or a second
                // warm-up) must not spawn a duplicate llama-server.
                let _build_guard = self.build_lock.lock().await;

                // Double-checked: a racing caller may have populated the
                // cache while we waited for the build lock.
                {
                    let guard = self.cached_local.lock().unwrap();
                    if let Some((cached_fingerprint, service)) = guard.as_ref() {
                        if *cached_fingerprint == fingerprint {
                            crate::log_info!("Reusing warm llama-server post-process service");
                            return Ok(service.clone());
                        }
                    }
                }

                // Spawning the sidecar takes seconds. Capture the generation
                // so an invalidation (disable, provider switch) landing
                // mid-build keeps the freshly built process out of the cache.
                let generation = self.generation.load(Ordering::SeqCst);
                let service = Arc::new(
                    SidecarPostProcess::new(
                        &config.post_process_engine,
                        &config.post_process_model,
                        &config.post_process_threads,
                        self.last_gpu_error.clone(),
                    )
                    .await
                    .map_err(|e| format!("Failed to start local post-process: {}", e))?,
                );

                if self.generation.load(Ordering::SeqCst) != generation {
                    // Dropped without caching: kill_on_drop terminates the
                    // process the caller was told not to want anymore.
                    return Err(
                        "Post-process configuration changed during sidecar startup".to_string()
                    );
                }

                let mut guard = self.cached_local.lock().unwrap();
                *guard = Some((fingerprint, service.clone()));
                Ok(service)
            }
        }
    }

    /// Drops the cached local sidecar, killing the llama-server process, so
    /// the next request starts a fresh one. Also invalidates any in-flight
    /// build via the generation counter. Used after failures, when local
    /// post-processing is disabled, and on factory reset.
    pub fn invalidate_local(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
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
