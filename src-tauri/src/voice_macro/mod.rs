pub mod executor;
pub mod matcher;
pub mod recognition;
pub mod sound;

pub use executor::{execute_macro_command, execute_macro_steps};
pub use matcher::{
    find_best_match, levenshtein_distance, match_phrase, normalize_phrase, string_similarity,
    MacroMatchResult,
};
pub use sound::{
    cleanup_orphaned_macro_sounds, clone_macro_sound, delete_macro_sound, import_macro_audio_file,
    macro_sound_path, play_macro_sound, play_macro_sound_file, play_macro_trigger_sound,
    save_macro_mic_recording,
};

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

use crate::app::state::SessionState;
use crate::config::TranscriptionMode;
use crate::engine_residency::EngineResidency;
use crate::AppState;

/// Whether the listener may detect speech right now. It pauses during a
/// dictation (so dictated speech never triggers a macro), until setup is
/// complete (so nothing listens behind the setup screen), and while the
/// local speech engine is unloaded (any nearby sound would otherwise reload
/// it moments after the user unloaded it).
fn listening_allowed(state: &AppState) -> bool {
    if *state.session_state.lock().unwrap() != SessionState::Idle {
        return false;
    }
    if !state.readiness.latest_is_ready() {
        return false;
    }
    let is_local = state.config.lock().unwrap().transcription_mode == TranscriptionMode::Local;
    !(is_local && state.engine_factory.residency() == EngineResidency::Unloaded)
}

static LISTENER_GENERATION: AtomicU64 = AtomicU64::new(1);

pub fn sync_voice_macro_listener(app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let (enabled, has_macros, active_macro_ids) = {
        let config = state.config.lock().unwrap();
        let ids: Vec<String> = config.voice_macros.iter().map(|m| m.id.clone()).collect();
        (
            config.voice_macros_enabled,
            !config.voice_macros.is_empty(),
            ids,
        )
    };

    let _ = sound::cleanup_orphaned_macro_sounds(&active_macro_ids);

    let mut cancel_guard = state.voice_macro_cancel.lock().unwrap();

    // 1. Always stop any previously running listener instance to avoid orphaned channels
    if let Some(cancel_tx) = cancel_guard.take() {
        crate::log_info!("Stopping previous Voice Macro background listener...");
        let _ = cancel_tx.send(());
    }

    // 2. If enabled and has macros, spawn a fresh listener instance attached to current engine
    if enabled && has_macros {
        crate::log_info!("Starting Voice Macro background listener...");
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<()>();
        *cancel_guard = Some(cancel_tx);

        let app = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            run_voice_macro_listener_loop(app, cancel_rx).await;
        });
    }
}

struct SpeechChunk {
    phrase: crate::audio::speech_segmenter::SpeechPhrase,
    detected_at: Instant,
}

async fn run_voice_macro_listener_loop(
    app_handle: AppHandle,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let my_gen = LISTENER_GENERATION.fetch_add(1, Ordering::SeqCst);
    let (audio_tx, audio_rx) = mpsc::sync_channel::<f32>(65536);
    let sample_rate;
    let engine_macro_tx;

    {
        let state = app_handle.state::<AppState>();
        // The capture owner (`app::capture`) starts the engine and re-syncs
        // this listener whenever it replaces it; the listener only attaches.
        let mut audio_guard = state.audio_engine.lock().unwrap();

        match audio_guard.as_mut() {
            Some(engine) => {
                sample_rate = engine.sample_rate;
                engine_macro_tx = engine.macro_tx.clone();
                *engine_macro_tx.lock().unwrap() = Some(audio_tx);
            }
            None => {
                crate::log_warn!(
                    "Voice Macro listener: Audio engine not available, exiting listener loop"
                );
                let mut cancel_guard = state.voice_macro_cancel.lock().unwrap();
                if LISTENER_GENERATION.load(Ordering::SeqCst) == my_gen + 1 {
                    *cancel_guard = None;
                }
                return;
            }
        }
    }

    let is_running = Arc::new(AtomicBool::new(true));
    let is_running_drain = is_running.clone();

    // Single-slot bounded channel to prevent queue backlog
    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel::<SpeechChunk>(1);
    let app_handle_for_vad = app_handle.clone();

    std::thread::spawn(move || {
        let mut segmenter = crate::audio::speech_segmenter::SpeechSegmenter::new(sample_rate);
        while is_running_drain.load(Ordering::Relaxed) {
            let sample = match audio_rx.recv_timeout(Duration::from_millis(30)) {
                Ok(sample) => sample,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            let Some(frame) = segmenter.push_sample(sample) else {
                continue;
            };
            let (threshold, may_listen) = {
                let state = app_handle_for_vad.state::<AppState>();
                let may_listen = listening_allowed(&state);
                let threshold = state
                    .config
                    .lock()
                    .unwrap()
                    .voice_macro_activation_threshold;
                (threshold, may_listen)
            };
            if !may_listen {
                // Paused (dictation running, setup incomplete or engine
                // asleep): drop any phrase in progress.
                segmenter.reset();
                continue;
            }
            let outcome = segmenter.process_frame(&frame, threshold);
            if outcome.level.speech_just_started {
                crate::log_info!(
                    "[Voice Macro VAD] Speech onset detected (level={:.4} >= threshold={:.4})",
                    outcome.level.smoothed_rms,
                    threshold
                );
            }
            if let Some(phrase) = outcome.phrase {
                crate::log_info!(
                    "[Voice Macro VAD] Phrase finalized ({:?}, duration={:.2}s, peak={:.4})",
                    phrase.ended_by,
                    phrase.duration_secs,
                    phrase.peak_level
                );
                let _ = chunk_tx.try_send(SpeechChunk {
                    phrase,
                    detected_at: Instant::now(),
                });
            }
        }
    });

    loop {
        tokio::select! {
            _ = &mut cancel_rx => {
                crate::log_info!("Voice Macro listener received cancel signal");
                break;
            }
            Some(chunk) = chunk_rx.recv() => {
                let queue_age = chunk.detected_at.elapsed();
                if queue_age > Duration::from_millis(1500) {
                    crate::log_warn!(
                        "[Voice Macro] Discarding stale audio chunk (age={:.1}ms)",
                        queue_age.as_secs_f64() * 1000.0
                    );
                    continue;
                }

                crate::log_info!(
                    "[Voice Macro] Processing speech chunk (duration={:.2}s, peak={:.4}, queue_age={:.1}ms)",
                    chunk.phrase.duration_secs,
                    chunk.phrase.peak_level,
                    queue_age.as_secs_f64() * 1000.0
                );

                let state = app_handle.state::<AppState>();
                let is_idle = {
                    let session = state.session_state.lock().unwrap();
                    *session == SessionState::Idle
                };

                if !is_idle {
                    continue;
                }

                let (enabled, has_macros, sound_feedback) = {
                    let config = state.config.lock().unwrap();
                    (
                        config.voice_macros_enabled,
                        !config.voice_macros.is_empty(),
                        config.voice_macro_sound_feedback,
                    )
                };
                if !enabled || !has_macros || !listening_allowed(&state) {
                    continue;
                }

                let recognize_start = Instant::now();
                let recognition =
                    match recognition::recognize_phrase(&state, &chunk.phrase.samples, sample_rate).await {
                        Ok(recognition) => recognition,
                        Err(e) => {
                            crate::log_warn!("Voice Macro recognition failed: {}", e);
                            continue;
                        }
                    };
                crate::log_info!(
                    "[Voice Macro] Transcription completed in {:.1}ms: \"{}\"",
                    recognize_start.elapsed().as_secs_f64() * 1000.0,
                    recognition.transcript
                );
                if recognition.transcript.is_empty() {
                    continue;
                }

                let match_result = recognition.match_result;
                let Some(matched_cmd) = match_result.matched_command.filter(|_| match_result.matched) else {
                    crate::log_info!(
                        "[Voice Macro] No macro matched for phrase: \"{}\" (best similarity: {:.1}%)",
                        recognition.transcript,
                        match_result.similarity * 100.0
                    );
                    continue;
                };
                crate::log_info!(
                    "[Voice Macro] MATCHED ({:.1}% match): \"{}\" -> \"{}\" ({} steps)",
                    match_result.similarity * 100.0,
                    recognition.transcript,
                    matched_cmd.phrase,
                    matched_cmd.resolve_steps().len()
                );

                play_macro_sound(&state, &matched_cmd, sound_feedback);

                let exec_start = Instant::now();
                if let Err(e) = execute_macro_command(&app_handle, &matched_cmd).await {
                    crate::log_warn!("Voice Macro execution failed: {}", e);
                } else {
                    crate::log_info!(
                        "[Voice Macro] ⚡ Executed in {:.1}ms (Total speech-end to key turnaround: {:.1}ms)",
                        exec_start.elapsed().as_secs_f64() * 1000.0,
                        chunk.detected_at.elapsed().as_secs_f64() * 1000.0
                    );
                }
            }
        }
    }

    is_running.store(false, Ordering::Relaxed);
    if LISTENER_GENERATION.load(Ordering::SeqCst) == my_gen + 1 {
        *engine_macro_tx.lock().unwrap() = None;
    }
    crate::log_info!("Voice Macro listener loop terminated");
}
