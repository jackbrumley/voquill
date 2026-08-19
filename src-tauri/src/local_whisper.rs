use crate::model_manager::ModelManager;
use crate::transcription::{TranscriptionError, TranscriptionService};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Hard ceiling on a single local transcription. If whisper.cpp doesn't finish
/// within this window, we surface an error instead of staying stuck.
const TRANSCRIBE_TIMEOUT_SECS: u64 = 180;
/// Hard ceiling on loading a model. If the GPU init hangs, we only bound how
/// long we wait; the spawned thread continues in the background.
const MODEL_LOAD_TIMEOUT_SECS: u64 = 120;

/// Monotonically increasing counter for transcribe calls, used to correlate
/// log entries with specific transcription invocations.
static TRANSCRIBE_CALL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A whisper model kept alive in memory across recordings. The context is the
/// loaded model file; a fresh `WhisperState` is created from it per call.
pub struct LoadedWhisperModel {
    pub context: WhisperContext,
    pub model_size: String,
    pub use_gpu: bool,
}

pub type WhisperEngineCache = Arc<Mutex<Option<LoadedWhisperModel>>>;

/// Drops whatever model is currently cached.
pub fn unload_model(cache: &WhisperEngineCache) {
    let mut guard = cache.lock().unwrap();
    *guard = None;
}

/// Runs a short silence inference to trigger GPU shader JIT compilation up
/// front so the first real recording doesn't pay that cost.
fn warm_up(context: &WhisperContext, model_size: &str, use_gpu: bool) {
    let silence = vec![0.0f32; 8000];
    let mut state = match context.create_state() {
        Ok(s) => s,
        Err(e) => {
            crate::log_warn!(
                "Warm-up: create_state failed for {} (gpu={}): {}",
                model_size,
                use_gpu,
                e
            );
            return;
        }
    };
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_suppress_nst(true);
    let start = Instant::now();
    match state.full(params, &silence) {
        Ok(()) => {
            let elapsed = start.elapsed();
            crate::log_info!("Model warm-up completed in {:?} (gpu={})", elapsed, use_gpu);
        }
        Err(e) => {
            crate::log_warn!("Model warm-up failed (gpu={}): {}", use_gpu, e);
        }
    }
}

/// Loads a model into the cache if one isn't already cached. Does NOT hold the
/// mutex during the blocking load so other callers can still inspect the cache.
fn ensure_model_loaded(
    cache: &WhisperEngineCache,
    model_path: &std::path::Path,
    model_size: &str,
    use_gpu: bool,
) -> Result<(), TranscriptionError> {
    {
        let guard = cache.lock().unwrap();
        if let Some(loaded) = guard.as_ref() {
            if loaded.model_size == model_size && loaded.use_gpu == use_gpu {
                return Ok(());
            }
        }
    }

    let path_str = model_path
        .to_str()
        .ok_or_else(|| TranscriptionError::Model("Invalid model path".to_string()))?
        .to_string();
    let ctx = WhisperContext::new_with_params(
        &path_str,
        WhisperContextParameters {
            use_gpu,
            ..Default::default()
        },
    )
    .map_err(|e| {
        TranscriptionError::Model(format!(
            "Failed to load model {} (gpu={}): {}",
            model_size, use_gpu, e
        ))
    })?;

    warm_up(&ctx, model_size, use_gpu);

    let mut guard = cache.lock().unwrap();
    *guard = Some(LoadedWhisperModel {
        context: ctx,
        model_size: model_size.to_string(),
        use_gpu,
    });
    crate::log_info!("Model {} cached (gpu={})", model_size, use_gpu);
    Ok(())
}

/// Same as `ensure_model_loaded` but with a wall-clock timeout on the blocking
/// load. Times out after `MODEL_LOAD_TIMEOUT_SECS` rather than hanging forever.
async fn ensure_model_loaded_with_timeout(
    cache: &WhisperEngineCache,
    model_path: PathBuf,
    model_size: String,
    use_gpu: bool,
) -> Result<(), TranscriptionError> {
    {
        let guard = cache.lock().unwrap();
        if let Some(loaded) = guard.as_ref() {
            if loaded.model_size == model_size && loaded.use_gpu == use_gpu {
                return Ok(());
            }
        }
    }

    let size_for_err = model_size.clone();
    let cache_clone = cache.clone();
    let handle = tokio::task::spawn_blocking(move || {
        ensure_model_loaded(&cache_clone, &model_path, &model_size, use_gpu)
    });

    match tokio::time::timeout(Duration::from_secs(MODEL_LOAD_TIMEOUT_SECS), handle).await {
        Ok(Ok(result)) => result,
        Ok(Err(join_error)) => Err(TranscriptionError::Model(format!(
            "Model loading thread panicked: {}",
            join_error
        ))),
        Err(_elapsed) => {
            crate::log_warn!(
                "Model loading timed out after {}s for {} (gpu={})",
                MODEL_LOAD_TIMEOUT_SECS,
                size_for_err,
                use_gpu
            );
            Err(TranscriptionError::Model(format!(
                "Model loading timed out after {}s",
                MODEL_LOAD_TIMEOUT_SECS
            )))
        }
    }
}

/// Describes which backend actually loaded and whether a fallback occurred.
pub struct LoadOutcome {
    pub use_gpu: bool,
    pub fell_back_from_gpu: Option<String>,
}

/// Tries to load the model on GPU first if requested; if that fails (timeout,
/// driver error, OOM), falls back to CPU and records the reason.
pub(crate) async fn ensure_model_loaded_with_fallback(
    cache: &WhisperEngineCache,
    model_path: PathBuf,
    model_size: String,
    use_gpu: bool,
    last_gpu_error: Option<&Arc<Mutex<Option<String>>>>,
) -> Result<LoadOutcome, TranscriptionError> {
    if !use_gpu {
        ensure_model_loaded_with_timeout(cache, model_path, model_size, false).await?;
        return Ok(LoadOutcome {
            use_gpu: false,
            fell_back_from_gpu: None,
        });
    }

    if let Some(err_slot) = &last_gpu_error {
        *err_slot.lock().unwrap() = None;
    }

    match ensure_model_loaded_with_timeout(cache, model_path.clone(), model_size.clone(), true)
        .await
    {
        Ok(()) => Ok(LoadOutcome {
            use_gpu: true,
            fell_back_from_gpu: None,
        }),
        Err(gpu_error) => {
            crate::log_warn!("GPU model load failed, falling back to CPU: {}", gpu_error);
            unload_model(cache);
            if let Some(err_slot) = last_gpu_error {
                *err_slot.lock().unwrap() = Some(gpu_error.to_string());
            }
            ensure_model_loaded_with_timeout(cache, model_path, model_size, false).await?;
            Ok(LoadOutcome {
                use_gpu: false,
                fell_back_from_gpu: Some(gpu_error.to_string()),
            })
        }
    }
}

pub struct LocalWhisperService {
    cache: WhisperEngineCache,
    model_size: String,
    model_path: PathBuf,
    use_gpu: bool,
    last_gpu_error: Option<Arc<Mutex<Option<String>>>>,
}

impl LocalWhisperService {
    pub fn new_full(
        cache: WhisperEngineCache,
        model_size: &str,
        use_gpu: bool,
        last_gpu_error: Option<Arc<Mutex<Option<String>>>>,
    ) -> Result<Self, TranscriptionError> {
        let model_manager = ModelManager::new().map_err(TranscriptionError::Model)?;
        let engine = if use_gpu {
            "Whisper.cpp (GPU)"
        } else {
            "Whisper.cpp"
        };
        let model = ModelManager::find_model(engine, model_size).ok_or_else(|| {
            TranscriptionError::Model(format!(
                "Model {} not found for engine {}",
                model_size, engine
            ))
        })?;
        let model_path = model_manager.get_model_path(&model);
        if !model_path.exists() {
            return Err(TranscriptionError::Model(format!(
                "Model {} not found. Please download it in settings.",
                model_size
            )));
        }
        Ok(Self {
            cache,
            model_size: model_size.to_string(),
            model_path,
            use_gpu,
            last_gpu_error,
        })
    }
}

#[async_trait]
impl TranscriptionService for LocalWhisperService {
    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<String, TranscriptionError> {
        let start_total = Instant::now();

        let mut reader = hound::WavReader::new(std::io::Cursor::new(audio_data))
            .map_err(|e| TranscriptionError::Audio(e.to_string()))?;

        let spec = reader.spec();
        if spec.channels != 1 || spec.sample_rate != 16000 {
            return Err(TranscriptionError::Audio(format!(
                "Unsupported audio format: {} channels, {}Hz. Expected 1 channel, 16000Hz.",
                spec.channels, spec.sample_rate
            )));
        }

        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / 32768.0))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TranscriptionError::Audio(e.to_string()))?;

        let model_path = self.model_path.clone();

        let outcome = ensure_model_loaded_with_fallback(
            &self.cache,
            model_path,
            self.model_size.clone(),
            self.use_gpu,
            self.last_gpu_error.as_ref(),
        )
        .await?;

        let actual_gpu = outcome.use_gpu;
        if let Some(ref reason) = outcome.fell_back_from_gpu {
            crate::log_warn!("GPU transcription unavailable, using CPU: {}", reason);
        }

        // ── Create a WhisperState from the cached context ──
        let mut state = {
            let guard = self.cache.lock().unwrap();
            let loaded = guard.as_ref().ok_or_else(|| {
                TranscriptionError::Model("Cache emptied unexpectedly".to_string())
            })?;
            loaded
                .context
                .create_state()
                .map_err(|e| TranscriptionError::Model(e.to_string()))?
        };

        let call_id = TRANSCRIBE_CALL_COUNTER.fetch_add(1, Ordering::SeqCst);
        let state_ptr = format!("{:p}", &state as *const _);
        crate::log_info!(
            "Transcribe call #{}: state={}, language={:?}, prompt_present={}, no_context=true",
            call_id,
            state_ptr,
            language,
            prompt.is_some(),
        );

        let owned_language = language.map(|l| l.to_string());
        let owned_prompt = prompt.map(|p| p.to_string());

        let load_elapsed = start_total.elapsed();

        let transcribe_start = Instant::now();
        let inference_handle = tokio::task::spawn_blocking(move || {
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            if let Some(ref lang) = owned_language {
                params.set_language(Some(lang.as_str()));
            }
            if let Some(ref p) = owned_prompt {
                params.set_initial_prompt(p.as_str());
            }
            params.set_translate(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_special(false);
            params.set_print_timestamps(false);
            params.set_no_context(true);
            params.set_suppress_nst(true);
            let run_result = state.full(params, &samples);
            match run_result {
                Ok(()) => {
                    let num_segments = state.full_n_segments();
                    let mut text = String::new();
                    for i in 0..num_segments {
                        if let Some(segment) = state.get_segment(i) {
                            if let Ok(segment_text) = segment.to_str_lossy() {
                                text.push_str(&segment_text);
                            }
                        }
                    }
                    Ok(text)
                }
                Err(e) => Err(TranscriptionError::Model(e.to_string())),
            }
        });

        let text = match tokio::time::timeout(
            Duration::from_secs(TRANSCRIBE_TIMEOUT_SECS),
            inference_handle,
        )
        .await
        {
            Ok(Ok(Ok(text))) => text,
            Ok(Ok(Err(e))) => return Err(e),
            Ok(Err(join_error)) => {
                return Err(TranscriptionError::Model(format!(
                    "Transcription thread panicked: {}",
                    join_error
                )));
            }
            Err(_elapsed) => {
                crate::log_warn!("Transcription timed out after {}s", TRANSCRIBE_TIMEOUT_SECS);
                return Err(TranscriptionError::Model(format!(
                    "Transcription timed out after {}s",
                    TRANSCRIBE_TIMEOUT_SECS
                )));
            }
        };

        let total_elapsed = start_total.elapsed();
        crate::log_info!(
            "Transcription ({}, gpu={}): load={:?}, inference={:?}, total={:?} chars={}",
            self.model_size,
            actual_gpu,
            load_elapsed,
            transcribe_start.elapsed(),
            total_elapsed,
            text.len(),
        );

        Ok(text.trim().to_string())
    }

    fn service_name(&self) -> &'static str {
        if self.use_gpu {
            "Local Whisper (GPU)"
        } else {
            "Local Whisper (CPU)"
        }
    }
}
