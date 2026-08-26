pub mod executor;
pub mod matcher;
pub mod sound;

pub use executor::{execute_macro_command, execute_macro_steps};
pub use matcher::{
    find_best_match, levenshtein_distance, match_phrase, normalize_phrase, string_similarity,
    MacroMatchResult,
};
pub use sound::{
    delete_macro_sound, import_macro_audio_file, macro_sound_path, play_macro_sound,
    play_macro_sound_file, play_macro_trigger_sound, save_macro_mic_recording,
};

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

use crate::app::state::SessionState;
use crate::audio::conversion::finalize_captured_audio_for_whisper;
use crate::AppState;

static LISTENER_GENERATION: AtomicU64 = AtomicU64::new(1);

pub fn sync_voice_macro_listener(app_handle: &AppHandle) {
    let state = app_handle.state::<AppState>();
    let (enabled, has_macros) = {
        let config = state.config.lock().unwrap();
        (config.voice_macros_enabled, !config.voice_macros.is_empty())
    };

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
    samples: Vec<f32>,
    duration_secs: f32,
    peak_rms: f32,
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
        let mut audio_guard = state.audio_engine.lock().unwrap();

        if audio_guard.is_none() {
            let requested_device = state.config.lock().unwrap().audio_device.clone();
            let resolved_device = match crate::audio::lookup_device(requested_device.clone()) {
                Ok(dev) => Some(dev),
                Err(e) => {
                    crate::log_warn!(
                        "Voice Macro listener: Failed to resolve audio device '{}': {}",
                        requested_device.unwrap_or_else(|| "default".to_string()),
                        e
                    );
                    None
                }
            };
            if let Some(dev) = resolved_device {
                let sensitivity = state.config.lock().unwrap().input_sensitivity;
                match crate::audio::PersistentAudioEngine::new(&dev, sensitivity) {
                    Ok(new_engine) => {
                        *audio_guard = Some(new_engine);
                        crate::log_info!(
                            "Voice Macro listener: Persistent audio engine initialized on-demand"
                        );
                    }
                    Err(e) => {
                        crate::log_warn!(
                            "Voice Macro listener: Failed to initialize audio engine on-demand: {}",
                            e
                        );
                    }
                }
            }
        }

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
        let frame_size = (sample_rate as f32 * 0.03) as usize; // 30ms window
        let pre_roll_frames = 7; // ~210ms pre-roll to catch speech onset
        let mut pre_roll_buffer: VecDeque<Vec<f32>> = VecDeque::with_capacity(pre_roll_frames);
        let mut frame_buffer = Vec::with_capacity(frame_size);
        let mut speech_buffer = Vec::with_capacity(sample_rate as usize * 4);
        let mut vad = crate::audio::vad::VoiceActivityDetector::new();
        let mut max_rms_in_phrase = 0.0f32;
        let max_speech_samples = (sample_rate as f32 * 4.0) as usize; // max 4.0s
        let min_speech_samples = (sample_rate as f32 * 0.30) as usize; // min 0.30s

        while is_running_drain.load(Ordering::Relaxed) {
            match audio_rx.recv_timeout(Duration::from_millis(30)) {
                Ok(sample) => {
                    frame_buffer.push(sample);
                    if frame_buffer.len() >= frame_size {
                        let open_threshold = {
                            let state = app_handle_for_vad.state::<AppState>();
                            let config = state.config.lock().unwrap();
                            config.voice_macro_activation_threshold
                        };

                        let vad_res = vad.process_frame(&frame_buffer, open_threshold);

                        if vad_res.speech_just_started {
                            speech_buffer.clear();
                            for pre in &pre_roll_buffer {
                                speech_buffer.extend_from_slice(pre);
                            }
                            speech_buffer.extend_from_slice(&frame_buffer);
                            max_rms_in_phrase = vad_res.smoothed_rms;
                            crate::log_info!(
                                "[Voice Macro VAD] Speech onset detected (RMS={:.4} >= threshold={:.4})",
                                vad_res.smoothed_rms,
                                open_threshold
                            );
                        } else if vad_res.is_speaking {
                            speech_buffer.extend_from_slice(&frame_buffer);
                            if vad_res.smoothed_rms > max_rms_in_phrase {
                                max_rms_in_phrase = vad_res.smoothed_rms;
                            }

                            if speech_buffer.len() >= max_speech_samples {
                                vad.reset();
                                let dur = speech_buffer.len() as f32 / sample_rate as f32;
                                crate::log_info!(
                                    "[Voice Macro VAD] Max duration reached, phrase finalized (dur={:.2}s, peak_rms={:.4})",
                                    dur,
                                    max_rms_in_phrase
                                );
                                let chunk = SpeechChunk {
                                    samples: speech_buffer.clone(),
                                    duration_secs: dur,
                                    peak_rms: max_rms_in_phrase,
                                    detected_at: Instant::now(),
                                };
                                let _ = chunk_tx.try_send(chunk);
                                max_rms_in_phrase = 0.0;
                                speech_buffer.clear();
                            }
                        } else if vad_res.speech_just_ended {
                            speech_buffer.extend_from_slice(&frame_buffer);
                            if speech_buffer.len() >= min_speech_samples {
                                let dur = speech_buffer.len() as f32 / sample_rate as f32;
                                crate::log_info!(
                                    "[Voice Macro VAD] Phrase finalized (duration={:.2}s, peak_rms={:.4}, samples={})",
                                    dur,
                                    max_rms_in_phrase,
                                    speech_buffer.len()
                                );

                                let chunk = SpeechChunk {
                                    samples: speech_buffer.clone(),
                                    duration_secs: dur,
                                    peak_rms: max_rms_in_phrase,
                                    detected_at: Instant::now(),
                                };
                                let _ = chunk_tx.try_send(chunk);
                            }
                            max_rms_in_phrase = 0.0;
                            speech_buffer.clear();
                        }

                        if pre_roll_buffer.len() >= pre_roll_frames {
                            pre_roll_buffer.pop_front();
                        }
                        pre_roll_buffer.push_back(frame_buffer.clone());
                        frame_buffer.clear();
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
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
                    "[Voice Macro] Processing speech chunk (duration={:.2}s, peak_rms={:.4}, queue_age={:.1}ms)",
                    chunk.duration_secs,
                    chunk.peak_rms,
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

                let (enabled, trigger_word, commands, sound_feedback, playback_dev) = {
                    let config = state.config.lock().unwrap();
                    (
                        config.voice_macros_enabled,
                        config.voice_macro_trigger_word.clone(),
                        config.voice_macros.clone(),
                        config.voice_macro_sound_feedback,
                        config.playback_device.clone(),
                    )
                };

                if !enabled || commands.is_empty() {
                    continue;
                }

                let wav_bytes = match finalize_captured_audio_for_whisper(&chunk.samples, sample_rate) {
                    Ok(b) => b,
                    Err(e) => {
                        crate::log_warn!("Voice Macro audio conversion failed: {}", e);
                        continue;
                    }
                };

                let config_snapshot = {
                    let config = state.config.lock().unwrap();
                    config.clone()
                };

                let service = match state.engine_factory.create_service(&config_snapshot).await {
                    Ok(s) => s,
                    Err(e) => {
                        crate::log_warn!("Voice Macro transcription service creation failed: {}", e);
                        continue;
                    }
                };

                let lang = if config_snapshot.language == "auto" {
                    None
                } else {
                    Some(config_snapshot.language.as_str())
                };

                let prompt_hint = config_snapshot.resolve_prompt_hint();
                let transcribe_start = Instant::now();
                match service.transcribe(&wav_bytes, lang, prompt_hint.as_deref()).await {
                    Ok(transcript) => {
                        let transcribe_time = transcribe_start.elapsed();
                        let trimmed = transcript.trim();
                        crate::log_info!(
                            "[Voice Macro] Transcription completed in {:.1}ms: \"{}\"",
                            transcribe_time.as_secs_f64() * 1000.0,
                            trimmed
                        );

                        if !trimmed.is_empty() {
                            let match_start = Instant::now();
                            let match_res = find_best_match(trimmed, &trigger_word, &commands);
                            if match_res.matched {
                                if let Some(matched_cmd) = match_res.matched_command {
                                    let match_time = match_start.elapsed();
                                    let steps = matched_cmd.resolve_steps();
                                    crate::log_info!(
                                        "[Voice Macro] MATCHED in {:.2}ms ({:.1}% match): \"{}\" -> \"{}\" ({} steps)",
                                        match_time.as_secs_f64() * 1000.0,
                                        match_res.similarity * 100.0,
                                        trimmed,
                                        matched_cmd.phrase,
                                        steps.len()
                                    );

                                    play_macro_sound(&matched_cmd, playback_dev, sound_feedback);

                                    let exec_start = Instant::now();
                                    if let Err(e) = execute_macro_command(&app_handle, &matched_cmd).await {
                                        crate::log_warn!("Voice Macro execution failed: {}", e);
                                    } else {
                                        let exec_time = exec_start.elapsed();
                                        let total_turnaround = chunk.detected_at.elapsed();
                                        crate::log_info!(
                                            "[Voice Macro] ⚡ Executed in {:.1}ms (Total speech-end to key turnaround: {:.1}ms)",
                                            exec_time.as_secs_f64() * 1000.0,
                                            total_turnaround.as_secs_f64() * 1000.0
                                        );
                                    }
                                }
                            } else {
                                crate::log_info!(
                                    "[Voice Macro] No macro matched for phrase: \"{}\" (best similarity: {:.1}%)",
                                    trimmed,
                                    match_res.similarity * 100.0
                                );
                            }
                        }
                    }
                    Err(e) => {
                        crate::log_warn!("Voice Macro background transcription error: {}", e);
                    }
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
