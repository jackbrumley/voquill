use crate::app::capture::{ensure_capture_engine, CaptureSettings};
use crate::app::commands::hotkey::re_register_hotkey;
use crate::config::Config;
use crate::{config, history, AppState};
use tauri::Emitter;

#[tauri::command]
pub async fn get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
pub async fn save_config(
    new_config: Config,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let mut normalized_config = new_config;
    normalized_config.normalize();

    let (hotkey_changed, previous_config, merged_config) = {
        let config_guard = state.config.lock().unwrap();
        let hotkey_changed = config_guard.hotkey != normalized_config.hotkey;
        let previous_config = config_guard.clone();

        let mut merged_config = normalized_config.clone();
        if merged_config.shortcuts_token.is_none() {
            merged_config.shortcuts_token = config_guard.shortcuts_token.clone();
        }
        if merged_config.input_token.is_none() {
            merged_config.input_token = config_guard.input_token.clone();
        }

        (hotkey_changed, previous_config, merged_config)
    };

    apply_audio_input_config(&app_handle, &state, &previous_config, &merged_config)?;

    {
        let mut config_guard = state.config.lock().unwrap();
        *config_guard = merged_config.clone();
    }

    if let Err(error) = config::storage::save_config(&merged_config) {
        return Err(format!("Failed to save config: {}", error));
    }

    reconcile_engine_warmup(&state, &previous_config, &merged_config, &app_handle);

    if hotkey_changed {
        if let Err(error) = re_register_hotkey(&app_handle, &merged_config.hotkey).await {
            let mut error_lock = state.hotkey_error.lock().unwrap();
            *error_lock = Some(error.clone());
            return Err(format!(
                "Config saved but failed to update hotkey: {}",
                error
            ));
        } else {
            let mut error_lock = state.hotkey_error.lock().unwrap();
            *error_lock = None;
        }
    }

    // Notify all windows (including the overlay) that config values changed.
    let _ = app_handle.emit("config-updated", ());

    crate::voice_macro::sync_voice_macro_listener(&app_handle);

    Ok(())
}

/// Applies a changed input device or sensitivity before the config is stored,
/// so an unusable selection is rejected instead of saved.
///
/// The mic-test lock is held while the capture engine is rebuilt.
/// `start_mic_test` takes the same lock before it touches the engine, so a
/// test started during a device change waits for it and records from the new
/// device, and a device change requested mid-test is rejected rather than
/// swapping the engine underneath the running test.
fn apply_audio_input_config(
    app_handle: &tauri::AppHandle,
    state: &AppState,
    previous_config: &Config,
    next_config: &Config,
) -> Result<(), String> {
    let input_changed = previous_config.audio_device != next_config.audio_device
        || previous_config.input_sensitivity != next_config.input_sensitivity;
    if !input_changed {
        return Ok(());
    }

    let is_mic_test_active = state.is_mic_test_active.lock().unwrap();
    if *is_mic_test_active {
        crate::log_warn!(
            "Audio config change rejected while mic test is active (requested_device='{}', sensitivity={:.2})",
            next_config.audio_device.as_deref().unwrap_or("default"),
            next_config.input_sensitivity
        );
        return Err(
            "Cannot change audio settings while mic test is active. Stop mic test and try again."
                .to_string(),
        );
    }

    ensure_capture_engine(app_handle, &CaptureSettings::from_config(next_config))
}

/// Reconciles engine warm-up state with a freshly saved config. The local
/// post-process sidecar is unloaded the moment local post-processing is
/// turned off, and both engines are pre-warmed when their settings change,
/// so the next dictation never pays the startup cost mid-session.
fn reconcile_engine_warmup(
    state: &AppState,
    previous_config: &Config,
    merged_config: &Config,
    app_handle: &tauri::AppHandle,
) {
    use crate::config::post_process::PostProcessProvider;
    use crate::config::TranscriptionMode;

    let was_local_post_process = previous_config.post_process_enabled
        && previous_config.post_process_provider == PostProcessProvider::Local;
    let is_local_post_process = merged_config.post_process_enabled
        && merged_config.post_process_provider == PostProcessProvider::Local;

    if was_local_post_process && !is_local_post_process {
        crate::log_info!("Local post-processing disabled; unloading llama-server sidecar");
        state.post_process_factory.invalidate_local();
    }

    let post_process_changed = previous_config.post_process_enabled
        != merged_config.post_process_enabled
        || previous_config.post_process_provider != merged_config.post_process_provider
        || previous_config.post_process_engine != merged_config.post_process_engine
        || previous_config.post_process_model != merged_config.post_process_model
        || previous_config.post_process_threads != merged_config.post_process_threads;

    if post_process_changed && is_local_post_process {
        crate::app::bootstrap::spawn_post_process_warmup(
            state.post_process_factory.clone(),
            merged_config,
            app_handle,
        );
    }

    let transcription_changed = previous_config.transcription_mode
        != merged_config.transcription_mode
        || previous_config.local_engine != merged_config.local_engine
        || previous_config.local_model_size != merged_config.local_model_size
        || previous_config.engine_config != merged_config.engine_config;

    if transcription_changed {
        // A running Parakeet sidecar belongs to the previous engine/model; the
        // preload below starts a new one if Parakeet is still selected.
        state.engine_factory.invalidate_parakeet();
    }

    if transcription_changed && merged_config.transcription_mode == TranscriptionMode::Local {
        crate::app::bootstrap::spawn_engine_preload(state.engine_factory.clone(), merged_config);
    }
}

#[tauri::command]
pub async fn reset_application_to_defaults(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::log_info!("Factory reset requested");

    // Stop the cached sidecars and unload models first so their processes
    // release the model/binary file locks before the models directory is
    // deleted.
    state.post_process_factory.invalidate_local();
    state.engine_factory.unload_all();

    let root_dir = crate::get_app_config_root_dir()?;

    let models_dir = root_dir.join("models");
    if models_dir.exists() {
        std::fs::remove_dir_all(&models_dir).map_err(|error| error.to_string())?;
    }
    std::fs::create_dir_all(&models_dir).map_err(|error| error.to_string())?;

    let debug_dir = root_dir.join("debug");
    std::fs::create_dir_all(&debug_dir).map_err(|error| error.to_string())?;
    crate::clear_directory_contents(&debug_dir, &["session.log", "last-session.log"])?;

    if let Err(error) = crate::truncate_session_log_with_header() {
        crate::log_warn!(
            "Could not truncate session log during factory reset: {}",
            error
        );
    }

    history::clear_history().map_err(|error| error.to_string())?;

    let default_config = Config::default();
    config::storage::save_config(&default_config)?;

    {
        let mut config_lock = state.config.lock().unwrap();
        *config_lock = default_config.clone();
    }

    {
        let mut mic_test_active = state.is_mic_test_active.lock().unwrap();
        *mic_test_active = false;
    }

    state.audio_player.stop_all();

    if let Err(error) =
        ensure_capture_engine(&app_handle, &CaptureSettings::from_config(&default_config))
    {
        crate::log_warn!(
            "Factory reset: capture engine not started on the default device: {}",
            error
        );
    }

    {
        let mut hotkey_error = state.hotkey_error.lock().unwrap();
        *hotkey_error = None;
    }

    if let Err(error) = re_register_hotkey(&app_handle, &default_config.hotkey).await {
        let mut hotkey_error = state.hotkey_error.lock().unwrap();
        *hotkey_error = Some(error.clone());
        return Err(format!(
            "Factory reset completed but failed to re-register default hotkey: {}",
            error
        ));
    }

    crate::app::status::emit_status_to_frontend("Ready").await;

    let _ = app_handle.emit("history-updated", serde_json::json!({ "items": [] }));
    let _ = app_handle.emit("config-updated", default_config.clone());
    let _ = app_handle.emit("setup-status-changed", serde_json::json!({}));

    crate::log_info!("Factory reset completed successfully");
    Ok(())
}
