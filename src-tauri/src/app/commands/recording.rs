use crate::{audio, AppState};
use tauri::Emitter;

#[tauri::command]
pub async fn start_recording(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let recording_before = *state.is_recording.lock().unwrap();
    crate::log_info!(
        "🎤 start_recording invoked: is_recording_before={}, configuring_hotkey={}",
        recording_before,
        *state.is_configuring_hotkey.lock().unwrap()
    );

    if *state.is_configuring_hotkey.lock().unwrap() {
        crate::log_info!("⚠️ Ignoring start_recording because hotkey configuration is active");
        return Err("Currently configuring hotkey".to_string());
    }

    let mut recording_flag = state.is_recording.lock().unwrap();
    if *recording_flag {
        return Err("Already recording".to_string());
    }

    *recording_flag = true;
    crate::log_info!(
        "🎤 start_recording command - Flag set true (before={}, after={})",
        recording_before,
        *recording_flag
    );

    let is_recording_clone = state.is_recording.clone();
    let config = state.config.clone();
    let app_handle_clone = app_handle.clone();
    let audio_engine = state.audio_engine.clone();

    {
        let mut engine_guard = audio_engine.lock().unwrap();
        if engine_guard.is_none() {
            crate::log_info!("🔧 Audio engine not found, attempting to initialize...");
            let cached_device = state.cached_device.lock().unwrap().clone();
            if let Some(device) = cached_device {
                let sensitivity = config.lock().unwrap().input_sensitivity;
                if let Ok(new_engine) = audio::PersistentAudioEngine::new(&device, sensitivity) {
                    *engine_guard = Some(new_engine);
                    crate::log_info!("✅ Audio engine initialized on demand");
                }
            }
        }
    }

    tokio::spawn(async move {
        crate::app::status::emit_status_update("Recording").await;
        let result = crate::app::recording_flow::record_and_transcribe(
            config,
            is_recording_clone,
            app_handle_clone,
            audio_engine,
        )
        .await;

        if let Err(error) = result {
            crate::log_info!("❌ Global Recording error: {}", error);
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut recording = state.is_recording.lock().unwrap();
    let before = *recording;
    *recording = false;
    crate::log_info!(
        "⏹️  stop_recording command - Flag set false (before={}, after={})",
        before,
        *recording
    );
    Ok(())
}

#[tauri::command]
pub async fn start_mic_test(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::log_info!("📡 Tauri Command: start_mic_test invoked");
    let mut mic_test_flag = state.is_mic_test_active.lock().unwrap();
    if *mic_test_flag {
        crate::log_info!("⚠️  start_mic_test: Already active");
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
            crate::log_info!("🔧 Audio engine not found for mic test, attempting to initialize...");
            let cached_device = state.cached_device.lock().unwrap().clone();
            if let Some(device) = cached_device {
                let sensitivity = state.config.lock().unwrap().input_sensitivity;
                if let Ok(new_engine) = audio::PersistentAudioEngine::new(&device, sensitivity) {
                    *engine_guard = Some(new_engine);
                    crate::log_info!("✅ Audio engine initialized on demand");
                }
            }
        }

        if engine_guard.is_none() {
            *mic_test_flag = false;
            return Err("Audio engine not initialized".to_string());
        }
    }

    tokio::spawn(async move {
        crate::log_info!("🎤 Mic test thread started");

        let sample_rate = {
            let guard = audio_engine.lock().unwrap();
            guard.as_ref().map(|engine| engine.sample_rate).unwrap_or(16000)
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
                crate::log_info!("✅ Mic test captured {} samples", captured_samples.len());
                if captured_samples.is_empty() {
                    crate::log_info!("⚠️  No audio captured, resetting UI...");
                    let _ = app_handle_clone.emit("mic-test-playback-finished", ());
                    return;
                }

                crate::log_info!("🔊 Initializing playback at {}Hz...", sample_rate);
                let app = app_handle_clone.clone();
                match audio::play_audio(captured_samples.clone(), sample_rate, move || {
                    crate::log_info!("🎵 Mic test playback finished");
                    let _ = app.emit("mic-test-playback-finished", ());
                }) {
                    Ok(stream) => {
                        let mut stream_guard = playback_stream_state.lock().unwrap();
                        *stream_guard = Some(stream);
                        crate::log_info!("✅ Playback stream active");
                        let _ = app_handle_clone.emit("mic-test-playback-started", ());
                    }
                    Err(error) => {
                        crate::log_info!("❌ Playback stream initialization failed: {}", error);
                        let _ = app_handle_clone.emit("mic-test-playback-finished", ());
                    }
                }

                let mut samples = mic_test_samples_clone.lock().unwrap();
                *samples = captured_samples;
            }
            Err(error) => {
                crate::log_info!("❌ Mic test recording error: {}", error);
                let _ = app_handle_clone.emit("mic-test-playback-finished", ());
            }
        }

        let mut mic_test_flag = is_mic_test_clone.lock().unwrap();
        if *mic_test_flag {
            *mic_test_flag = false;
            crate::log_info!("🔧 Mic test active flag reset after mic test completion");
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_mic_test(state: tauri::State<'_, AppState>) -> Result<(), String> {
    crate::log_info!("📡 Tauri Command: stop_mic_test invoked");
    let mut mic_test_flag = state.is_mic_test_active.lock().unwrap();
    *mic_test_flag = false;
    crate::log_info!("⏹️  Mic test flag set to false");
    Ok(())
}

#[tauri::command]
pub async fn stop_mic_playback(state: tauri::State<'_, AppState>) -> Result<(), String> {
    crate::log_info!("📡 Tauri Command: stop_mic_playback invoked");
    let mut stream_guard = state.playback_stream.lock().unwrap();
    *stream_guard = None;
    crate::log_info!("⏹️  Playback stopped by user");
    Ok(())
}
