use crate::app::state::SessionState;
use crate::{audio, AppState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

#[tauri::command]
pub async fn start_recording(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let session_before = *state.session_state.lock().unwrap();
    crate::log_info!(
        "start_recording invoked: session_state={:?}, configuring_hotkey={}",
        session_before,
        *state.is_configuring_hotkey.lock().unwrap()
    );

    if *state.is_configuring_hotkey.lock().unwrap() {
        crate::log_info!("Ignoring start_recording because hotkey configuration is active");
        return Err("Currently configuring hotkey".to_string());
    }

    {
        let session = state.session_state.lock().unwrap();
        if *session != SessionState::Idle {
            return Err("Session already active".to_string());
        }
    }

    // Mark the session as Recording up front so a hotkey release landing
    // while the audio engine initializes is still observed as a stop.
    {
        let mut session = state.session_state.lock().unwrap();
        *session = SessionState::Recording;
    }
    crate::app::status::emit_status_update("Recording").await;

    let requested_device = { state.config.lock().unwrap().audio_device.clone() };
    let engine_initialized_or_ready = {
        let mut engine_guard = state.audio_engine.lock().unwrap();
        if engine_guard.is_some() {
            true
        } else {
            crate::log_info!("Audio engine not found, attempting to initialize...");
            let resolved_device = {
                let cached_device = state.cached_device.lock().unwrap().clone();
                if cached_device.is_some() {
                    cached_device
                } else {
                    match audio::lookup_device(requested_device.clone()) {
                        Ok(device) => {
                            crate::log_info!(
                                "Resolved input device on demand for recording (requested_device='{}')",
                                requested_device
                                    .clone()
                                    .unwrap_or_else(|| "default".to_string())
                            );
                            let mut cache_guard = state.cached_device.lock().unwrap();
                            *cache_guard = Some(device.clone());
                            Some(device)
                        }
                        Err(error) => {
                            crate::log_warn!(
                                "Failed to resolve input device for recording (requested_device='{}'): {}",
                                requested_device
                                    .clone()
                                    .unwrap_or_else(|| "default".to_string()),
                                error
                            );
                            None
                        }
                    }
                }
            };

            if let Some(device) = resolved_device {
                let sensitivity = state.config.lock().unwrap().input_sensitivity;
                match audio::PersistentAudioEngine::new(&device, sensitivity) {
                    Ok(new_engine) => {
                        *engine_guard = Some(new_engine);
                        crate::log_info!("Audio engine initialized on demand");
                        true
                    }
                    Err(error) => {
                        crate::log_warn!(
                            "Failed to initialize audio engine on demand for recording: {}",
                            error
                        );
                        false
                    }
                }
            } else {
                crate::log_warn!(
                    "Audio engine initialization skipped for recording: input device unresolved"
                );
                false
            }
        }
    };

    if !engine_initialized_or_ready {
        crate::log_info!("Recording cannot start: no audio device available");
        *state.session_state.lock().unwrap() = SessionState::Idle;
        crate::app::status::emit_status_to_frontend("Error").await;
        return Ok(());
    }

    // If a release (or toggle stop) landed while the engine was initializing,
    // stop_recording already moved the session on; do not start a capture.
    if *state.session_state.lock().unwrap() != SessionState::Recording {
        crate::log_info!(
            "start_recording: session left Recording during engine init; aborting start"
        );
        *state.session_state.lock().unwrap() = SessionState::Idle;
        crate::app::status::emit_status_to_frontend("Ready").await;
        return Ok(());
    }

    let session_token = Arc::new(AtomicBool::new(false));
    {
        let mut active_session = state.active_session.lock().unwrap();
        *active_session = Some(session_token.clone());
    }
    crate::log_info!("start_recording: capture pipeline starting");

    let session_state = state.session_state.clone();
    let config = state.config.clone();
    let app_handle_clone = app_handle.clone();
    let audio_engine = state.audio_engine.clone();
    let engine_factory = state.engine_factory.clone();

    tokio::spawn(async move {
        crate::log_info!("Recording task started");
        let result = crate::app::recording_flow::record_and_transcribe(
            config,
            session_state,
            session_token,
            app_handle_clone,
            audio_engine,
            engine_factory,
        )
        .await;

        if let Err(error) = result {
            crate::log_info!("Global Recording error: {}", error);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut session = state.session_state.lock().unwrap();
    if *session == SessionState::Recording {
        *session = SessionState::Transcribing;
        crate::log_info!("stop_recording: Recording -> Transcribing (capture will finalize)");
    } else {
        crate::log_info!("stop_recording: ignored in session state {:?}", *session);
    }

    Ok(())
}

/// Cancels the in-flight dictation session, discarding any captured audio and
/// transcription output. Triggered by pressing the hotkey while a session is
/// active past the recording phase.
pub async fn cancel_session(state: tauri::State<'_, AppState>) {
    let session = *state.session_state.lock().unwrap();
    if session == SessionState::Idle {
        return;
    }

    crate::log_info!(
        "cancel_session: cancelling session from state {:?}",
        session
    );
    if let Some(token) = state.active_session.lock().unwrap().as_ref() {
        token.store(true, Ordering::SeqCst);
    }

    match session {
        SessionState::Recording => {
            // End capture promptly. The pipeline observes the cancel token,
            // discards the audio, and finishes the session (Idle + Ready).
            // active_session stays attached so no new capture can overlap the
            // one that is still unwinding.
            *state.session_state.lock().unwrap() = SessionState::Transcribing;
        }
        SessionState::Transcribing | SessionState::Typing => {
            // Capture already finished, so it is safe to detach immediately:
            // the in-flight pipeline discards its result and cannot clobber a
            // newer session because its token no longer matches.
            *state.active_session.lock().unwrap() = None;
            *state.session_state.lock().unwrap() = SessionState::Idle;
            crate::app::status::emit_status_to_frontend("Ready").await;
        }
        SessionState::Idle => {}
    }
}

#[tauri::command]
pub async fn start_mic_test(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::log_info!("Tauri Command: start_mic_test invoked");
    let mut mic_test_flag = state.is_mic_test_active.lock().unwrap();
    if *mic_test_flag {
        crate::log_info!("start_mic_test: Already active");
        return Err("Mic test already active".to_string());
    }
    *mic_test_flag = true;

    let mut samples = state.mic_test_samples.lock().unwrap();
    samples.clear();

    let is_mic_test_clone = state.is_mic_test_active.clone();
    let mic_test_samples_clone = state.mic_test_samples.clone();
    let audio_engine = state.audio_engine.clone();
    let playback_stream_state = state.playback_stream.clone();
    let app_handle_clone = app_handle.clone();

    {
        let mut engine_guard = audio_engine.lock().unwrap();
        if engine_guard.is_none() {
            crate::log_info!("Audio engine not found for mic test, attempting to initialize...");
            let requested_device = { state.config.lock().unwrap().audio_device.clone() };

            let resolved_device = {
                let cached_device = state.cached_device.lock().unwrap().clone();
                if cached_device.is_some() {
                    cached_device
                } else {
                    match audio::lookup_device(requested_device.clone()) {
                        Ok(device) => {
                            crate::log_info!(
                                "Resolved input device on demand for mic test (requested_device='{}')",
                                requested_device
                                    .clone()
                                    .unwrap_or_else(|| "default".to_string())
                            );
                            let mut cache_guard = state.cached_device.lock().unwrap();
                            *cache_guard = Some(device.clone());
                            Some(device)
                        }
                        Err(error) => {
                            crate::log_warn!(
                                "Failed to resolve input device for mic test (requested_device='{}'): {}",
                                requested_device
                                    .clone()
                                    .unwrap_or_else(|| "default".to_string()),
                                error
                            );
                            None
                        }
                    }
                }
            };

            if let Some(device) = resolved_device {
                let sensitivity = state.config.lock().unwrap().input_sensitivity;
                match audio::PersistentAudioEngine::new(&device, sensitivity) {
                    Ok(new_engine) => {
                        *engine_guard = Some(new_engine);
                        crate::log_info!("Audio engine initialized on demand");
                    }
                    Err(error) => {
                        crate::log_warn!(
                            "Failed to initialize audio engine on demand for mic test: {}",
                            error
                        );
                    }
                }
            } else {
                crate::log_warn!(
                    "Audio engine initialization skipped for mic test: input device unresolved"
                );
            }
        }

        if engine_guard.is_none() {
            *mic_test_flag = false;
            return Err("Audio engine not initialized".to_string());
        }
    }

    tokio::spawn(async move {
        crate::log_info!("Mic test thread started");

        let sample_rate = {
            let guard = audio_engine.lock().unwrap();
            guard
                .as_ref()
                .map(|engine| engine.sample_rate)
                .unwrap_or(16000)
        };

        let result = audio::record_mic_test(&is_mic_test_clone, audio_engine, {
            let app = app_handle_clone.clone();
            move |volume| {
                let _ = app.emit("mic-test-volume", volume);
            }
        })
        .await;

        match result {
            Ok(captured_samples) => {
                crate::log_info!("Mic test captured {} samples", captured_samples.len());
                if captured_samples.is_empty() {
                    crate::log_info!("No audio captured, resetting UI...");
                    let _ = app_handle_clone.emit("mic-test-playback-finished", ());
                    return;
                }

                crate::log_info!("Initializing playback at {}Hz...", sample_rate);
                let app = app_handle_clone.clone();
                match audio::play_audio(captured_samples.clone(), sample_rate, move || {
                    crate::log_info!("Mic test playback finished");
                    let _ = app.emit("mic-test-playback-finished", ());
                }) {
                    Ok(stream) => {
                        let mut stream_guard = playback_stream_state.lock().unwrap();
                        *stream_guard = Some(stream);
                        crate::log_info!("Playback stream active");
                        let _ = app_handle_clone.emit("mic-test-playback-started", ());
                    }
                    Err(error) => {
                        crate::log_info!("Playback stream initialization failed: {}", error);
                        let _ = app_handle_clone.emit("mic-test-playback-finished", ());
                    }
                }

                let mut samples = mic_test_samples_clone.lock().unwrap();
                *samples = captured_samples;
            }
            Err(error) => {
                crate::log_info!("Mic test recording error: {}", error);
                let _ = app_handle_clone.emit("mic-test-playback-finished", ());
            }
        }

        let mut mic_test_flag = is_mic_test_clone.lock().unwrap();
        if *mic_test_flag {
            *mic_test_flag = false;
            crate::log_info!("Mic test active flag reset after mic test completion");
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_mic_test(state: tauri::State<'_, AppState>) -> Result<(), String> {
    crate::log_info!("Tauri Command: stop_mic_test invoked");
    let mut mic_test_flag = state.is_mic_test_active.lock().unwrap();
    *mic_test_flag = false;
    crate::log_info!("Mic test flag set to false");
    Ok(())
}

#[tauri::command]
pub async fn stop_mic_playback(state: tauri::State<'_, AppState>) -> Result<(), String> {
    crate::log_info!("Tauri Command: stop_mic_playback invoked");
    let mut stream_guard = state.playback_stream.lock().unwrap();
    *stream_guard = None;
    crate::log_info!("Playback stopped by user");
    Ok(())
}
