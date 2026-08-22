pub mod audio_processing;
pub mod output;

use crate::app::state::SessionState;
use crate::config::Config;
use crate::{audio, engine_factory, typing};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

pub async fn record_and_transcribe(
    config: Arc<Mutex<Config>>,
    session_state: Arc<Mutex<SessionState>>,
    session_token: Arc<AtomicBool>,
    app_handle: AppHandle,
    audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
    engine_factory: Arc<engine_factory::EngineFactory>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let result = record_and_transcribe_inner(
        &config,
        &session_state,
        &session_token,
        &app_handle,
        audio_engine,
        &engine_factory,
    )
    .await;
    finish_session(&app_handle, &session_state, &session_token).await;
    result
}

/// Returns the session to Idle and the status to Ready, but only if this
/// pipeline still owns the active session. A cancelled-and-replaced pipeline
/// must never clobber the state or status of a newer session.
async fn finish_session(
    app_handle: &AppHandle,
    session_state: &Arc<Mutex<SessionState>>,
    session_token: &Arc<AtomicBool>,
) {
    let state = app_handle.state::<crate::AppState>();
    let is_active_session = {
        let active_session = state.active_session.lock().unwrap();
        active_session
            .as_ref()
            .is_some_and(|token| Arc::ptr_eq(token, session_token))
    };
    if !is_active_session {
        crate::log_info!("finish_session: session was superseded; leaving state untouched");
        return;
    }
    *state.active_session.lock().unwrap() = None;
    *session_state.lock().unwrap() = SessionState::Idle;
    crate::app::status::emit_status_to_frontend("Ready").await;
}

async fn record_and_transcribe_inner(
    config: &Arc<Mutex<Config>>,
    session_state: &Arc<Mutex<SessionState>>,
    session_token: &Arc<AtomicBool>,
    app_handle: &AppHandle,
    audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
    engine_factory: &Arc<engine_factory::EngineFactory>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (post_roll_ms, max_recording_duration) = {
        let config_guard = config.lock().unwrap();
        (
            config_guard.post_roll_ms,
            std::time::Duration::from_secs(
                config_guard
                    .max_recording_duration_minutes
                    .saturating_mul(60),
            ),
        )
    };

    let audio_data = audio::record_audio_while_flag(
        session_state,
        audio_engine,
        post_roll_ms,
        max_recording_duration,
    )
    .await?;

    // Capture has ended, however it ended (release, toggle stop, cancel, or
    // the max-duration auto-stop): the recording phase is over.
    {
        let mut session = session_state.lock().unwrap();
        if *session == SessionState::Recording {
            *session = SessionState::Transcribing;
        }
    }

    if session_token.load(Ordering::SeqCst) {
        crate::log_info!("Session cancelled during capture; discarding audio");
        return Ok(());
    }

    crate::app::status::emit_status_to_frontend("Transcribing").await;

    if audio_data.is_empty() {
        return Ok(());
    }
    if let Err(error) = audio_processing::validate_audio_duration(&audio_data) {
        crate::log_info!("Audio validation failed: {}", error);
        return Ok(());
    }

    let (lang_code_str, prompt_hint, current_config) = {
        let config_guard = config.lock().unwrap();
        (
            config_guard.language.clone(),
            config_guard.resolve_prompt_hint(),
            config_guard.clone(),
        )
    };

    if session_token.load(Ordering::SeqCst) {
        crate::log_info!("Session cancelled before transcription; discarding audio");
        return Ok(());
    }

    let lang_code = if lang_code_str == "auto" {
        None
    } else {
        Some(lang_code_str.as_str())
    };

    if let Some(ref hint) = prompt_hint {
        crate::log_info!("Transcription prompt hint: \"{}\"", hint);
    }

    // Apply audio noise reduction if enabled (before transcription)
    let audio_data = if current_config.noise_reduction_enabled {
        crate::app::status::emit_status_to_frontend("Processing").await;
        match audio_processing::run_noise_reduction(
            app_handle,
            &audio_data,
            current_config.noise_reduction_strength,
        )
        .await
        {
            Ok(enhanced) => {
                crate::log_info!(
                    "Noise reduction applied ({} bytes -> {} bytes)",
                    audio_data.len(),
                    enhanced.len()
                );
                enhanced
            }
            Err(e) => {
                crate::log_warn!("Noise reduction failed, using raw audio: {}", e);
                audio_data
            }
        }
    } else {
        audio_data
    };

    let service = engine_factory.create_service(&current_config).await;
    let service = match service {
        Ok(s) => s,
        Err(error) => {
            crate::log_info!("Failed to create transcription service: {}", error);
            return Err(error.into());
        }
    };

    // ── Transcription: per-segment or full-file ──
    let mut diar_segments: Vec<crate::diarization::Segment> = Vec::new();
    let text = if current_config.diarization_enabled_recording {
        // Save the recorded audio to a temp file for diarization
        let temp_dir = crate::paths::temp_dir();
        let _ = std::fs::create_dir_all(&temp_dir);
        let temp_path = temp_dir.join(format!(
            "recording_{}.wav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        if let Err(e) = std::fs::write(&temp_path, &audio_data) {
            crate::log_warn!("Failed to save temp audio for diarization: {}", e);
            // Fall back to full-file transcription
            let text = match service
                .transcribe(&audio_data, lang_code, prompt_hint.as_deref())
                .await
            {
                Ok(t) => t,
                Err(error) => {
                    crate::log_info!(
                        "Transcription failed ({}): {}",
                        service.service_name(),
                        error
                    );
                    return Err(error.into());
                }
            };
            crate::log_info!(
                "Transcription received ({}): \"{}\"",
                service.service_name(),
                text
            );
            text
        } else {
            let diar_result = audio_processing::run_diarization_for_recording(
                app_handle,
                &temp_path,
                current_config.diarization_cluster_threshold,
            )
            .await;
            let _ = std::fs::remove_file(&temp_path);

            match diar_result {
                Ok(diar) if !diar.segments.is_empty() => {
                    crate::log_info!(
                        "Diarization returned {} segments from {}",
                        diar.segments.len(),
                        diar.provider
                    );

                    let mut segment_pairs: Vec<(Option<String>, String)> = Vec::new();

                    for seg in &diar.segments {
                        let start = seg.start_sec.unwrap_or(0.0);
                        let end = seg.end_sec.unwrap_or(1000000.0);

                        let seg_wav = match audio::extract_segment_wav(&audio_data, start, end) {
                            Ok(w) => w,
                            Err(e) => {
                                crate::log_warn!("Failed to extract segment, skipping: {}", e);
                                continue;
                            }
                        };

                        let seg_text = match service
                            .transcribe(&seg_wav, lang_code, prompt_hint.as_deref())
                            .await
                        {
                            Ok(t) => t,
                            Err(e) => {
                                crate::log_warn!("Segment transcription failed, skipping: {}", e);
                                continue;
                            }
                        };
                        crate::log_info!(
                            "Segment [{}]: \"{}\"",
                            seg.speaker.as_deref().unwrap_or("?"),
                            seg_text
                        );
                        segment_pairs.push((seg.speaker.clone(), seg_text));
                    }

                    if segment_pairs.is_empty() {
                        String::new()
                    } else {
                        let unique_speakers: std::collections::HashSet<Option<&str>> =
                            segment_pairs.iter().map(|(s, _)| s.as_deref()).collect();

                        if unique_speakers.len() <= 1 {
                            segment_pairs
                                .iter()
                                .map(|(_, t)| t.as_str())
                                .collect::<Vec<_>>()
                                .join(" ")
                        } else {
                            diar_segments = segment_pairs
                                .iter()
                                .map(|(s, t)| crate::diarization::Segment {
                                    speaker: s.clone(),
                                    text: t.clone(),
                                    start_sec: None,
                                    end_sec: None,
                                })
                                .collect();

                            segment_pairs
                                .iter()
                                .map(|(s, t)| {
                                    format!("[{}] {}", s.as_deref().unwrap_or("Speaker"), t)
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        }
                    }
                }
                Ok(_) => {
                    crate::log_warn!(
                        "Diarization returned 0 segments — transcribing full recording"
                    );
                    let text = match service
                        .transcribe(&audio_data, lang_code, prompt_hint.as_deref())
                        .await
                    {
                        Ok(t) => t,
                        Err(error) => {
                            crate::log_info!(
                                "Transcription failed ({}): {}",
                                service.service_name(),
                                error
                            );
                            return Err(error.into());
                        }
                    };
                    crate::log_info!(
                        "Transcription received ({}): \"{}\"",
                        service.service_name(),
                        text
                    );
                    text
                }
                Err(e) => {
                    crate::log_warn!("Diarization failed, transcribing full recording: {}", e);
                    let text = match service
                        .transcribe(&audio_data, lang_code, prompt_hint.as_deref())
                        .await
                    {
                        Ok(t) => t,
                        Err(error) => {
                            crate::log_info!(
                                "Transcription failed ({}): {}",
                                service.service_name(),
                                error
                            );
                            return Err(error.into());
                        }
                    };
                    crate::log_info!(
                        "Transcription received ({}): \"{}\"",
                        service.service_name(),
                        text
                    );
                    text
                }
            }
        }
    } else {
        let text = match service
            .transcribe(&audio_data, lang_code, prompt_hint.as_deref())
            .await
        {
            Ok(t) => t,
            Err(error) => {
                crate::log_info!(
                    "Transcription failed ({}): {}",
                    service.service_name(),
                    error
                );
                return Err(error.into());
            }
        };
        crate::log_info!(
            "Transcription received ({}): \"{}\"",
            service.service_name(),
            text
        );
        text
    };

    // Apply regex-based filler word removal, works without LLM post-processing
    let text = crate::text_cleanup::clean_transcription(
        &text,
        current_config.filler_word_removal_enabled,
        &current_config.custom_filler_words,
    );

    // Save the raw (pre-post-process) text for history display
    let raw_text = if current_config.post_process_enabled {
        Some(text.clone())
    } else {
        None
    };

    let text = if !text.trim().is_empty() && current_config.post_process_enabled {
        crate::log_info!("Post-processing transcription...");
        let post_process_factory = app_handle
            .state::<crate::AppState>()
            .post_process_factory
            .clone();
        match post_process_factory.get_service(&current_config).await {
            Ok(processor) => {
                crate::app::status::emit_status_to_frontend("Processing").await;
                match processor
                    .post_process(
                        &text,
                        &current_config.resolve_post_process_prompt(),
                        &current_config.resolve_user_prompt_template(),
                        current_config.resolve_max_output_tokens(),
                    )
                    .await
                {
                    Ok(cleaned) => {
                        crate::log_info!(
                            "Post-processed ({}): \"{}\"",
                            processor.service_name(),
                            cleaned
                        );
                        cleaned
                    }
                    Err(e) => {
                        crate::log_warn!("Post-processing failed, using raw text: {}", e);
                        // A network error from the local sidecar means the
                        // process is gone or unreachable; drop it so the next
                        // dictation respawns a fresh one.
                        if matches!(e, crate::post_process::PostProcessError::Network(_)) {
                            post_process_factory.invalidate_local();
                        }
                        text
                    }
                }
            }
            Err(e) => {
                crate::log_warn!(
                    "Could not create post-process service, using raw text: {}",
                    e
                );
                text
            }
        }
    } else {
        text
    };

    let text = typing::normalize_for_typing(&text);

    // Apply trailing space if configured
    let (append_trailing_space, auto_submit) = {
        let config_guard = config.lock().unwrap();
        (config_guard.append_trailing_space, config_guard.auto_submit)
    };
    let output_text = if append_trailing_space && !text.is_empty() {
        format!("{} ", text)
    } else {
        text
    };

    if session_token.load(Ordering::SeqCst) {
        crate::log_info!("Session cancelled during transcription; discarding result");
        return Ok(());
    }

    output::deliver_output(
        app_handle,
        session_state,
        session_token,
        config,
        output::OutputPayload {
            output_text,
            diar_segments,
            raw_text,
            auto_submit,
        },
    )
    .await
}
