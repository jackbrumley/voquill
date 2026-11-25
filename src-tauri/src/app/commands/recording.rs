use crate::app::capture::{ensure_capture_engine, CaptureSettings};
use crate::app::state::SessionState;
use crate::{audio, AppState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager};

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

    // The cached verdict keeps the hotkey instant; a cached "not ready" is
    // re-checked so a stale one never blocks a working setup.
    if !state.readiness.latest_is_ready() {
        let status = crate::app::readiness::evaluate(&app_handle).await;
        if !status.is_all_ready {
            let reason = format!("Setup is not complete: {}", status.reasons.join("; "));
            // Show the window so the user lands on setup and sees what is left.
            crate::log_info!("Ignoring start_recording: {}", reason);
            crate::app::tray::show_main_window(&app_handle);
            return Err(reason);
        }
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

    let capture_settings = CaptureSettings::from_config(&state.config.lock().unwrap());
    if let Err(error) = ensure_capture_engine(&app_handle, &capture_settings) {
        crate::log_warn!("Recording cannot start: {}", error);
        *state.session_state.lock().unwrap() = SessionState::Idle;
        crate::app::capture::sync_idle_capture(&app_handle);
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
        crate::app::capture::sync_idle_capture(&app_handle);
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
    let should_emit = {
        let mut session = state.session_state.lock().unwrap();
        if *session == SessionState::Recording {
            *session = SessionState::Transcribing;
            crate::log_info!("stop_recording: Recording -> Transcribing (capture will finalize)");
            true
        } else {
            crate::log_info!("stop_recording: ignored in session state {:?}", *session);
            false
        }
    };

    if should_emit {
        crate::app::status::emit_status_update("Transcribing").await;
    }

    Ok(())
}

/// Cancels the in-flight dictation session, discarding any captured audio and
/// transcription output. Triggered by pressing the hotkey while a session is
/// active past the recording phase.
pub async fn cancel_session(state: tauri::State<'_, AppState>, app_handle: tauri::AppHandle) {
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
            crate::app::capture::sync_idle_capture(&app_handle);
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

    // The mic-test flag stays locked until the test is running, so a
    // concurrent `save_config` cannot swap the engine underneath it.
    let capture_settings = CaptureSettings::from_config(&state.config.lock().unwrap());
    if let Err(error) = ensure_capture_engine(&app_handle, &capture_settings) {
        *mic_test_flag = false;
        crate::log_warn!("Mic test cannot start: {}", error);
        return Err(error);
    }

    let is_mic_test_clone = state.is_mic_test_active.clone();
    let audio_engine = state.audio_engine.clone();
    let app_handle_clone = app_handle.clone();
    let config_arc = state.config.clone();

    tokio::spawn(async move {
        crate::log_info!("Mic test thread started");

        let sample_rate = {
            let guard = audio_engine.lock().unwrap();
            guard
                .as_ref()
                .map(|engine| engine.sample_rate)
                .unwrap_or(16000)
        };

        let threshold = {
            let config = config_arc.lock().unwrap();
            config.voice_macro_activation_threshold
        };

        let result = audio::record_mic_test(&is_mic_test_clone, audio_engine, threshold, {
            let app = app_handle_clone.clone();
            move |payload| {
                let _ = app.emit("mic-test-volume", payload);
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

                let level = audio::level::measure(&captured_samples, sample_rate);
                let clip =
                    prepare_mic_test_playback(&app_handle_clone, &captured_samples, sample_rate)
                        .await;
                crate::log_info!("Playing mic test back at {}Hz", clip.sample_rate);
                let _ = app_handle_clone.emit(
                    "mic-test-playback-started",
                    MicTestPlaybackPayload {
                        is_too_quiet: level.is_too_quiet,
                    },
                );
                let finished_handle = app_handle_clone.clone();
                let playback = app_handle_clone.state::<AppState>().play_audio(
                    audio::PlaybackSlot::Preview,
                    clip,
                    move || {
                        crate::log_info!("Mic test playback ended");
                        let _ = finished_handle.emit("mic-test-playback-finished", ());
                    },
                );
                if let Err(error) = playback {
                    crate::log_warn!("Mic test playback failed: {}", error);
                    let _ = app_handle_clone.emit("mic-test-playback-finished", ());
                }
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
        drop(mic_test_flag);
        crate::app::capture::sync_idle_capture(&app_handle_clone);
    });

    Ok(())
}

/// What the mic test tells the UI as playback starts. The backend decides
/// whether the microphone is too quiet, from the same level rules every
/// recording uses.
#[derive(Clone, serde::Serialize)]
struct MicTestPlaybackPayload {
    is_too_quiet: bool,
}

/// Prepares the mic test recording exactly as a dictation would be (level,
/// trimming, and noise reduction when it is on), so the user hears what the
/// speech engine hears. Any step that fails falls back to the raw capture,
/// logged.
async fn prepare_mic_test_playback(
    app_handle: &tauri::AppHandle,
    captured_samples: &[f32],
    sample_rate: u32,
) -> audio::AudioClip {
    let raw_capture = || audio::AudioClip {
        samples: captured_samples.to_vec(),
        sample_rate,
    };
    let mut wav_data =
        match audio::conversion::finalize_captured_audio_for_whisper(captured_samples, sample_rate)
        {
            Ok(wav_data) => wav_data,
            Err(error) => {
                crate::log_warn!("Failed to prepare mic test audio: {}", error);
                return raw_capture();
            }
        };

    let (noise_reduction_enabled, noise_reduction_strength) = {
        let state = app_handle.state::<AppState>();
        let config = state.config.lock().unwrap();
        (
            config.noise_reduction_enabled,
            config.noise_reduction_strength,
        )
    };
    if noise_reduction_enabled {
        crate::log_info!(
            "Applying noise reduction (strength={:.2}) to mic test audio...",
            noise_reduction_strength
        );
        match crate::app::recording_flow::audio_processing::run_noise_reduction(
            app_handle,
            &wav_data,
            noise_reduction_strength,
        )
        .await
        {
            Ok(enhanced_wav) => wav_data = enhanced_wav,
            Err(error) => crate::log_warn!("Noise reduction failed during mic test: {}", error),
        }
    }

    match audio::decode::decode_compressed_audio(&wav_data) {
        Ok(decoded) => audio::AudioClip {
            samples: decoded.samples,
            sample_rate: decoded.sample_rate,
        },
        Err(error) => {
            crate::log_warn!("Failed to decode prepared mic test audio: {}", error);
            raw_capture()
        }
    }
}

#[tauri::command]
pub async fn stop_mic_test(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::log_info!("Tauri Command: stop_mic_test invoked");
    let mut mic_test_flag = state.is_mic_test_active.lock().unwrap();
    *mic_test_flag = false;
    crate::log_info!("Mic test flag set to false");
    drop(mic_test_flag);
    // Capture may still be draining / playing back; the mic-test task calls
    // sync_idle_capture when it fully finishes. Nudge now in case it already ended.
    crate::app::capture::sync_idle_capture(&app_handle);
    Ok(())
}

#[tauri::command]
pub async fn stop_mic_playback(state: tauri::State<'_, AppState>) -> Result<(), String> {
    crate::log_info!("Tauri Command: stop_mic_playback invoked");
    state.audio_player.stop(audio::PlaybackSlot::Preview);
    Ok(())
}

/// Plays a history recording on the configured playback device. Emits
/// `history-playback-finished` with the file name when it ends for any reason
/// (played out, stopped, or replaced by another preview).
#[tauri::command]
pub async fn play_history_recording(
    file_name: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    crate::log_info!(
        "Tauri Command: play_history_recording invoked ({})",
        file_name
    );
    let path = history_recording_path(&file_name)?;
    let wav_bytes = std::fs::read(&path)
        .map_err(|error| format!("Failed to read {}: {}", path.display(), error))?;
    let clip = audio::AudioClip::from_wav_bytes(&wav_bytes)?;

    state.play_audio(audio::PlaybackSlot::Preview, clip, move || {
        crate::log_info!("History playback ended ({})", file_name);
        let _ = app_handle.emit("history-playback-finished", file_name);
    })
}

#[tauri::command]
pub async fn stop_history_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    crate::log_info!("Tauri Command: stop_history_recording invoked");
    state.audio_player.stop(audio::PlaybackSlot::Preview);
    Ok(())
}

/// Recordings live in the debug recordings directory; older builds wrote them
/// to the debug directory itself.
fn history_recording_path(file_name: &str) -> Result<std::path::PathBuf, String> {
    let is_plain_file_name = std::path::Path::new(file_name)
        .file_name()
        .is_some_and(|name| name == file_name);
    if !is_plain_file_name {
        return Err(format!("Invalid recording file name '{}'", file_name));
    }

    let recording_path = crate::paths::debug_recordings_dir()?.join(file_name);
    if recording_path.exists() {
        return Ok(recording_path);
    }
    let legacy_path = crate::paths::debug_dir()?.join(file_name);
    if legacy_path.exists() {
        return Ok(legacy_path);
    }
    Err(format!("Recording audio file '{}' not found", file_name))
}

#[cfg(test)]
mod tests {
    use super::history_recording_path;

    #[test]
    fn history_recording_path_rejects_anything_but_a_file_name() {
        for name in ["../settings.json", "sub/dir.wav", "/etc/passwd", "..", ""] {
            let error = history_recording_path(name).unwrap_err();
            assert!(
                error.starts_with("Invalid recording file name"),
                "{name}: {error}"
            );
        }
    }
}
