pub mod audio_processing;
pub mod output;

use crate::app::state::SessionState;
use crate::config::Config;
use crate::{audio, engine_factory, typing};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

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

struct SessionContext<'a> {
    session_uuid: &'a str,
    saved_audio_file: Option<&'a str>,
    duration_secs: f64,
    lang_code_str: &'a str,
    prompt_name: Option<&'a str>,
    history_limit: usize,
}

async fn transcribe_full_audio(
    service: &(dyn crate::transcription::TranscriptionService + Send + Sync),
    audio_data: &[u8],
    lang_code: Option<&str>,
    prompt_hint: Option<&str>,
    ctx: &SessionContext<'_>,
    app_handle: &AppHandle,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    match service.transcribe(audio_data, lang_code, prompt_hint).await {
        Ok(text) => {
            crate::log_info!(
                "[session:{}] Transcription received ({}): \"{}\"",
                &ctx.session_uuid[..8],
                service.service_name(),
                text
            );
            Ok(text)
        }
        Err(error) => {
            crate::log_info!(
                "[session:{}] Transcription failed ({}): {}",
                &ctx.session_uuid[..8],
                service.service_name(),
                error
            );
            let _ = crate::history::add_history_item(&crate::history::NewHistoryItem {
                session_uuid: ctx.session_uuid,
                status: "failed",
                text: "",
                raw_text: None,
                error_message: Some(&format!("Transcription failed: {}", error)),
                segments: None,
                audio_file: ctx.saved_audio_file,
                duration_secs: Some(ctx.duration_secs),
                engine: Some(service.service_name()),
                source: Some("mic"),
                language: Some(ctx.lang_code_str),
                prompt_name: ctx.prompt_name,
                limit: Some(ctx.history_limit),
            });
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.emit("history-updated", ());
            }
            Err(error.into())
        }
    }
}

async fn record_and_transcribe_inner(
    config: &Arc<Mutex<Config>>,
    session_state: &Arc<Mutex<SessionState>>,
    session_token: &Arc<AtomicBool>,
    app_handle: &AppHandle,
    audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
    engine_factory: &Arc<engine_factory::EngineFactory>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session_uuid = uuid::Uuid::new_v4().to_string();
    crate::log_info!("[session:{}] Recording flow started", &session_uuid[..8]);

    let (post_roll_ms, max_recording_duration, current_config, history_limit) = {
        let config_guard = config.lock().unwrap();
        (
            config_guard.post_roll_ms,
            std::time::Duration::from_secs(
                config_guard
                    .max_recording_duration_minutes
                    .saturating_mul(60),
            ),
            config_guard.clone(),
            config_guard.history_limit,
        )
    };
    let lang_code_str = current_config.language.clone();
    let prompt_hint = current_config.resolve_prompt_hint();
    let prompt_name = current_config.resolve_post_process_prompt_name();

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
        crate::log_info!(
            "[session:{}] Session cancelled during capture; discarding audio",
            &session_uuid[..8]
        );
        let _ = crate::history::add_history_item(&crate::history::NewHistoryItem {
            session_uuid: &session_uuid,
            status: "cancelled",
            text: "",
            raw_text: None,
            error_message: Some("Session cancelled during capture"),
            segments: None,
            audio_file: None,
            duration_secs: None,
            engine: None,
            source: Some("mic"),
            language: Some(&lang_code_str),
            prompt_name: prompt_name.as_deref(),
            limit: Some(history_limit),
        });
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("history-updated", ());
        }
        return Ok(());
    }

    crate::app::status::emit_status_to_frontend("Transcribing").await;

    if audio_data.is_empty() {
        return Ok(());
    }

    let duration_secs = match audio_processing::validate_audio_duration(&audio_data) {
        Ok(d) => d,
        Err(error) => {
            crate::log_info!(
                "[session:{}] Audio validation failed: {}",
                &session_uuid[..8],
                error
            );
            let _ = crate::history::add_history_item(&crate::history::NewHistoryItem {
                session_uuid: &session_uuid,
                status: "failed",
                text: "",
                raw_text: None,
                error_message: Some(&format!("Audio validation failed: {}", error)),
                segments: None,
                audio_file: None,
                duration_secs: None,
                engine: None,
                source: Some("mic"),
                language: Some(&lang_code_str),
                prompt_name: prompt_name.as_deref(),
                limit: Some(history_limit),
            });
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.emit("history-updated", ());
            }
            return Ok(());
        }
    };

    let saved_audio_file = if current_config.enable_recording_logs {
        match crate::paths::debug_recordings_dir() {
            Ok(dir) => {
                let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                let file_name = format!("recording_{}_{}.wav", timestamp, &session_uuid[..8]);
                let file_path = dir.join(&file_name);
                if let Err(e) = std::fs::write(&file_path, &audio_data) {
                    crate::log_warn!(
                        "[session:{}] Failed to save debug recording: {}",
                        &session_uuid[..8],
                        e
                    );
                    None
                } else {
                    crate::log_info!(
                        "[session:{}] Debug recording saved: {:?}",
                        &session_uuid[..8],
                        file_path
                    );
                    Some(file_name)
                }
            }
            Err(e) => {
                crate::log_warn!(
                    "[session:{}] Failed to get debug recordings directory: {}",
                    &session_uuid[..8],
                    e
                );
                None
            }
        }
    } else {
        None
    };

    if session_token.load(Ordering::SeqCst) {
        crate::log_info!(
            "[session:{}] Session cancelled before transcription; discarding audio",
            &session_uuid[..8]
        );
        let _ = crate::history::add_history_item(&crate::history::NewHistoryItem {
            session_uuid: &session_uuid,
            status: "cancelled",
            text: "",
            raw_text: None,
            error_message: Some("Session cancelled before transcription"),
            segments: None,
            audio_file: saved_audio_file.as_deref(),
            duration_secs: Some(duration_secs),
            engine: None,
            source: Some("mic"),
            language: Some(&lang_code_str),
            prompt_name: prompt_name.as_deref(),
            limit: Some(history_limit),
        });
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("history-updated", ());
        }
        return Ok(());
    }

    let lang_code = if lang_code_str == "auto" {
        None
    } else {
        Some(lang_code_str.as_str())
    };

    if let Some(ref hint) = prompt_hint {
        crate::log_info!(
            "[session:{}] Transcription prompt hint: \"{}\"",
            &session_uuid[..8],
            hint
        );
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
                    "[session:{}] Noise reduction applied ({} bytes -> {} bytes)",
                    &session_uuid[..8],
                    audio_data.len(),
                    enhanced.len()
                );
                enhanced
            }
            Err(e) => {
                crate::log_warn!(
                    "[session:{}] Noise reduction failed, using raw audio: {}",
                    &session_uuid[..8],
                    e
                );
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
            crate::log_info!(
                "[session:{}] Failed to create transcription service: {}",
                &session_uuid[..8],
                error
            );
            let _ = crate::history::add_history_item(&crate::history::NewHistoryItem {
                session_uuid: &session_uuid,
                status: "failed",
                text: "",
                raw_text: None,
                error_message: Some(&format!("Failed to create service: {}", error)),
                segments: None,
                audio_file: saved_audio_file.as_deref(),
                duration_secs: Some(duration_secs),
                engine: Some(&current_config.local_engine),
                source: Some("mic"),
                language: Some(&lang_code_str),
                prompt_name: prompt_name.as_deref(),
                limit: Some(history_limit),
            });
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.emit("history-updated", ());
            }
            return Err(error.into());
        }
    };

    let session_ctx = SessionContext {
        session_uuid: &session_uuid,
        saved_audio_file: saved_audio_file.as_deref(),
        duration_secs,
        lang_code_str: &lang_code_str,
        prompt_name: prompt_name.as_deref(),
        history_limit,
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
            transcribe_full_audio(
                service.as_ref(),
                &audio_data,
                lang_code,
                prompt_hint.as_deref(),
                &session_ctx,
                app_handle,
            )
            .await?
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
                    transcribe_full_audio(
                        service.as_ref(),
                        &audio_data,
                        lang_code,
                        prompt_hint.as_deref(),
                        &session_ctx,
                        app_handle,
                    )
                    .await?
                }
                Err(e) => {
                    crate::log_warn!("Diarization failed, transcribing full recording: {}", e);
                    transcribe_full_audio(
                        service.as_ref(),
                        &audio_data,
                        lang_code,
                        prompt_hint.as_deref(),
                        &session_ctx,
                        app_handle,
                    )
                    .await?
                }
            }
        }
    } else {
        transcribe_full_audio(
            service.as_ref(),
            &audio_data,
            lang_code,
            prompt_hint.as_deref(),
            &session_ctx,
            app_handle,
        )
        .await?
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
        crate::log_info!(
            "[session:{}] Session cancelled during transcription; discarding result",
            &session_uuid[..8]
        );
        let _ = crate::history::add_history_item(&crate::history::NewHistoryItem {
            session_uuid: &session_uuid,
            status: "cancelled",
            text: "",
            raw_text: raw_text.as_deref(),
            error_message: Some("Session cancelled during transcription/post-processing"),
            segments: None,
            audio_file: saved_audio_file.as_deref(),
            duration_secs: Some(duration_secs),
            engine: Some(service.service_name()),
            source: Some("mic"),
            language: Some(&lang_code_str),
            prompt_name: prompt_name.as_deref(),
            limit: Some(history_limit),
        });
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("history-updated", ());
        }
        return Ok(());
    }

    output::deliver_output(
        app_handle,
        session_state,
        session_token,
        config,
        output::OutputPayload {
            session_uuid,
            output_text,
            diar_segments,
            raw_text,
            auto_submit,
            audio_file: saved_audio_file,
            duration_secs: Some(duration_secs),
            engine: Some(service.service_name().to_string()),
        },
    )
    .await
}
