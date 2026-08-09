use crate::app::state::SessionState;
use crate::config::{Config, OutputMethod};
use crate::{audio, engine_factory, history, typing};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

fn validate_audio_duration(
    audio_data: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if audio_data.len() < 44 {
        return Err("Audio file too small".into());
    }
    let sample_rate = u32::from_le_bytes([
        audio_data[24],
        audio_data[25],
        audio_data[26],
        audio_data[27],
    ]);
    let channels = u16::from_le_bytes([audio_data[22], audio_data[23]]);
    let bits_per_sample = u16::from_le_bytes([audio_data[34], audio_data[35]]);

    let mut data_size = 0u32;
    let mut pos = 36;
    while pos + 8 <= audio_data.len() {
        let chunk_id = &audio_data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            audio_data[pos + 4],
            audio_data[pos + 5],
            audio_data[pos + 6],
            audio_data[pos + 7],
        ]);
        if chunk_id == b"data" {
            data_size = chunk_size;
            break;
        }
        pos += 8 + chunk_size as usize;
        if chunk_size % 2 == 1 {
            pos += 1;
        }
    }

    if data_size == 0 {
        return Err("No data chunk".into());
    }
    let bytes_per_sample = (bits_per_sample / 8) as u32;
    let bytes_per_second = sample_rate * channels as u32 * bytes_per_sample;
    let duration_seconds = data_size as f64 / bytes_per_second as f64;

    crate::log_info!("Audio duration: {:.3}s", duration_seconds);
    if duration_seconds < 0.1 {
        return Err("Audio too short".into());
    }
    Ok(())
}

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
    if let Err(error) = validate_audio_duration(&audio_data) {
        crate::log_info!("Audio validation failed: {}", error);
        return Ok(());
    }

    let (enable_recording_logs, language_choice) = {
        let config_guard = config.lock().unwrap();
        (
            config_guard.enable_recording_logs,
            config_guard.language.clone(),
        )
    };

    let dictionary_words = {
        let config_guard = config.lock().unwrap();
        config_guard.dictionary.clone()
    };

    let lang_code = match language_choice.as_str() {
        "auto" => None,
        "en-AU" => Some("en"),
        "en-GB" => Some("en"),
        "en-US" => Some("en"),
        code => Some(code),
    };

    let mut prompt_hint: Option<String> = match language_choice.as_str() {
        "en-AU" => Some("Australian spelling.".to_string()),
        "en-GB" => Some("British spelling.".to_string()),
        "en-US" => Some("American spelling.".to_string()),
        _ => None,
    };

    if !dictionary_words.is_empty() {
        let dict_str = dictionary_words.join(", ");
        prompt_hint = match prompt_hint {
            Some(hint) => Some(format!("{}, {}", hint, dict_str)),
            None => Some(dict_str),
        };
    }

    if enable_recording_logs {
        let debug_path = dirs::config_dir()
            .unwrap_or_default()
            .join("foss-voquill")
            .join("debug")
            .join("recordings")
            .join(format!(
                "recording_{}.wav",
                ::chrono::Local::now().format("%Y%m%d_%H%M%S")
            ));

        if let Err(error) = std::fs::create_dir_all(debug_path.parent().unwrap()) {
            crate::log_info!("Failed to create debug directory: {}", error);
        } else if let Err(error) = std::fs::write(&debug_path, &audio_data) {
            crate::log_info!("Failed to save debug recording: {}", error);
        } else {
            crate::log_info!("Debug recording saved to: {:?}", debug_path);
        }
    }

    crate::log_info!("Language: {:?}, Hint: {:?}", lang_code, prompt_hint);

    let current_config = {
        let guard = config.lock().unwrap();
        guard.clone()
    };
    let service = engine_factory.create_service(&current_config).await;
    let service = match service {
        Ok(s) => s,
        Err(error) => {
            crate::log_info!("Failed to create transcription service: {}", error);
            return Err(error.into());
        }
    };

    let text = match service
        .transcribe(&audio_data, lang_code, prompt_hint.as_deref())
        .await
    {
        Ok(text) => {
            crate::log_info!(
                "Transcription received ({}): \"{}\"",
                service.service_name(),
                text
            );
            text
        }

        Err(error) => {
            crate::log_info!(
                "Transcription failed ({}): {}",
                service.service_name(),
                error
            );
            return Err(error.into());
        }
    };

    let text = if !text.trim().is_empty() && current_config.post_process_enabled {
        crate::log_info!("Post-processing transcription...");
        match crate::post_process::factory::PostProcessFactory::create_service(&current_config)
            .await
        {
            Ok(processor) => {
                crate::app::status::emit_status_to_frontend("Processing").await;
                match processor.post_process(&text).await {
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

    if session_token.load(Ordering::SeqCst) {
        crate::log_info!("Session cancelled during transcription; discarding result");
        return Ok(());
    }

    if !text.trim().is_empty() {
        let _ = history::add_history_item(&text);
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("history-updated", ());
        }

        {
            let mut session = session_state.lock().unwrap();
            *session = SessionState::Typing;
        }
        crate::app::status::emit_status_to_frontend("Typing").await;
        let (typing_speed, hold_duration, output_method, copy_on_typewriter) = {
            let config_guard = config.lock().unwrap();
            (
                config_guard.typing_speed_interval,
                config_guard.key_press_duration_ms,
                config_guard.output_method.clone(),
                config_guard.copy_on_typewriter,
            )
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        if session_token.load(Ordering::SeqCst) {
            crate::log_info!("Session cancelled before typing; discarding output");
            return Ok(());
        }

        match output_method {
            OutputMethod::Typewriter => {
                if copy_on_typewriter {
                    if let Err(error) = typing::copy_to_clipboard(&text) {
                        crate::log_info!("CLIPBOARD ERROR: {}", error);
                    }
                }
                crate::log_info!("Forwarding text to hardware typing engine...");
                let state = app_handle.state::<crate::AppState>();
                if let Err(error) = state
                    .display_backend
                    .type_text_hardware(app_handle, &text, typing_speed, hold_duration)
                    .await
                {
                    crate::log_info!("TYPING ENGINE ERROR: {}", error);
                }
            }
            OutputMethod::Clipboard => {
                crate::log_info!("Copying text to clipboard (Clipboard Mode)...");
                if let Err(error) = typing::copy_to_clipboard(&text) {
                    crate::log_info!("CLIPBOARD ERROR: {}", error);
                }
            }
        }
    } else {
        crate::log_info!("Transcription was empty, skipping typing.");
    }

    Ok(())
}
