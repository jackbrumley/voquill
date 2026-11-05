//! Keeps the Parakeet sherpa-onnx sidecar running across dictations.
//!
//! Starting the sidecar costs a process spawn plus a full model load (about
//! 2 s), so one warm process is kept per configuration, like the Whisper model
//! cache. `EngineFactory` owns the cache: it warms it on preload, hands out
//! clones of the cached service, and invalidates it when the engine or model
//! changes or models are unloaded.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use super::ParakeetService;
use crate::transcription::TranscriptionError;

/// The configuration a sidecar was started with. Any change starts a new one.
#[derive(Clone, PartialEq)]
struct SidecarFingerprint {
    model_dir: PathBuf,
    num_threads: usize,
}

struct CachedSidecar {
    fingerprint: SidecarFingerprint,
    service: ParakeetService,
}

pub struct ParakeetSidecarCache {
    slot: Mutex<Option<CachedSidecar>>,
    /// Serializes sidecar starts so a warm-up racing a dictation cannot spawn
    /// a second sherpa-onnx process.
    build_lock: tokio::sync::Mutex<()>,
    /// Bumped by `invalidate`; a start that began before an invalidation must
    /// not populate the cache when it finishes.
    generation: AtomicU64,
}

impl Default for ParakeetSidecarCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ParakeetSidecarCache {
    pub fn new() -> Self {
        Self {
            slot: Mutex::new(None),
            build_lock: tokio::sync::Mutex::new(()),
            generation: AtomicU64::new(0),
        }
    }

    /// Returns the warm sidecar for this configuration, starting one only when
    /// none is cached, the configuration changed, or the cached process exited.
    pub async fn service(
        &self,
        model_dir: PathBuf,
        model_size: &str,
        num_threads: usize,
    ) -> Result<ParakeetService, TranscriptionError> {
        let fingerprint = SidecarFingerprint {
            model_dir: model_dir.clone(),
            num_threads,
        };

        if let Some(service) = self.live_cached(&fingerprint).await {
            return Ok(service);
        }

        let _build_guard = self.build_lock.lock().await;

        // A racing caller may have started the sidecar while we waited.
        if let Some(service) = self.live_cached(&fingerprint).await {
            return Ok(service);
        }

        let generation = self.generation.load(Ordering::SeqCst);
        let service = ParakeetService::new(model_dir, model_size, num_threads).await?;

        if self.generation.load(Ordering::SeqCst) == generation {
            // Replacing a stale entry drops its clone; that process exits once
            // any in-flight transcription still holding it finishes.
            *self.slot.lock().unwrap() = Some(CachedSidecar {
                fingerprint,
                service: service.clone(),
            });
        } else {
            crate::log_info!("Parakeet sidecar invalidated while starting; not caching it");
        }
        Ok(service)
    }

    /// Drops the cached sidecar, which stops the sherpa-onnx process once no
    /// in-flight transcription still holds it, and keeps any start already in
    /// progress out of the cache.
    pub fn invalidate(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        if self.slot.lock().unwrap().take().is_some() {
            crate::log_info!("Parakeet sidecar stopped");
        }
    }

    /// The cached service if it matches `fingerprint` and its process is still
    /// running. A dead process is evicted so the caller starts a new one.
    async fn live_cached(&self, fingerprint: &SidecarFingerprint) -> Option<ParakeetService> {
        let cached = {
            let slot = self.slot.lock().unwrap();
            match slot.as_ref() {
                Some(entry) if entry.fingerprint == *fingerprint => Some(entry.service.clone()),
                _ => None,
            }
        }?;

        if cached.is_alive().await {
            return Some(cached);
        }

        crate::log_warn!("Cached Parakeet sidecar has exited; starting a new one");
        let mut slot = self.slot.lock().unwrap();
        if slot
            .as_ref()
            .is_some_and(|entry| entry.service.shares_process_with(&cached))
        {
            *slot = None;
        }
        None
    }
}
