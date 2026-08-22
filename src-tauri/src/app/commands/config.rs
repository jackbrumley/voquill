use crate::app::commands::hotkey::re_register_hotkey;
use crate::config::Config;
use crate::{audio, config, history, AppState};
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

    let is_mic_test_active = *state.is_mic_test_active.lock().unwrap();

    let (restart_engine, hotkey_changed, previous_config, merged_config) = {
        let config_guard = state.config.lock().unwrap();
        let audio_changed = config_guard.audio_device != normalized_config.audio_device
            || config_guard.input_sensitivity != normalized_config.input_sensitivity;
        let hotkey_changed = config_guard.hotkey != normalized_config.hotkey;
        let previous_config = config_guard.clone();

        let mut merged_config = normalized_config.clone();
        if merged_config.shortcuts_token.is_none() {
            merged_config.shortcuts_token = config_guard.shortcuts_token.clone();
        }
        if merged_config.input_token.is_none() {
            merged_config.input_token = config_guard.input_token.clone();
        }

        (
            audio_changed,
            hotkey_changed,
            previous_config,
            merged_config,
        )
    };

    let mut prepared_device: Option<cpal::Device> = None;
    let mut prepared_engine: Option<audio::PersistentAudioEngine> = None;

    if restart_engine {
        if is_mic_test_active {
            let selected_device = merged_config
                .audio_device
                .clone()
                .unwrap_or_else(|| "default".to_string());
            crate::log_warn!(
                "Audio config change rejected while mic test is active (requested_device='{}', sensitivity={:.2})",
                selected_device,
                merged_config.input_sensitivity
            );
            return Err(
                "Cannot change audio settings while mic test is active. Stop mic test and try again."
                    .to_string(),
            );
        }

        let selected_device = merged_config.audio_device.clone();
        let resolved_device = audio::lookup_device(selected_device.clone()).map_err(|error| {
            format!(
                "Failed to resolve input device '{}': {}",
                selected_device.unwrap_or_else(|| "default".to_string()),
                error
            )
        })?;

        crate::log_info!(
            "Audio config changed, validating persistent engine restart (requested_device='{}', sensitivity={:.2})",
            merged_config
                .audio_device
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            merged_config.input_sensitivity
        );

        let new_engine = audio::PersistentAudioEngine::new(
            &resolved_device,
            merged_config.input_sensitivity,
        )
        .map_err(|error| {
            format!(
                "Failed to initialize persistent audio engine for device '{}' (sensitivity {:.2}): {}",
                merged_config
                    .audio_device
                    .clone()
                    .unwrap_or_else(|| "default".to_string()),
                merged_config.input_sensitivity,
                error
            )
        })?;

        prepared_device = Some(resolved_device);
        prepared_engine = Some(new_engine);
    }

    {
        let mut config_guard = state.config.lock().unwrap();
        *config_guard = merged_config.clone();
    }

    if restart_engine {
        {
            let mut cached_device = state.cached_device.lock().unwrap();
            *cached_device = prepared_device;
        }
        {
            let mut engine_guard = state.audio_engine.lock().unwrap();
            *engine_guard = prepared_engine;
        }
        crate::log_info!("Persistent engine restarted");
    } else {
        let cached = match audio::lookup_device(merged_config.audio_device.clone()) {
            Ok(device) => Some(device),
            Err(error) => {
                crate::log_warn!(
                    "Failed to pre-warm audio device cache (requested_device='{}'): {}",
                    merged_config
                        .audio_device
                        .clone()
                        .unwrap_or_else(|| "default".to_string()),
                    error
                );
                None
            }
        };
        let mut cached_device = state.cached_device.lock().unwrap();
        *cached_device = cached;
        crate::log_info!("Pre-warmed audio device cache");
    }

    if let Err(error) = config::save_config(&merged_config) {
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

    Ok(())
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
    use crate::config::{PostProcessProvider, TranscriptionMode};

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
        || previous_config.local_model_size != merged_config.local_model_size;

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

    // Stop the cached post-process sidecar first so its process releases the
    // model/binary file locks before the models directory is deleted.
    state.post_process_factory.invalidate_local();

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
    config::save_config(&default_config).map_err(|error| error.to_string())?;

    {
        let mut config_lock = state.config.lock().unwrap();
        *config_lock = default_config.clone();
    }

    {
        let mut cached_device = state.cached_device.lock().unwrap();
        *cached_device = None;
    }

    {
        let mut mic_test_active = state.is_mic_test_active.lock().unwrap();
        *mic_test_active = false;
    }

    {
        let mut playback_stream = state.playback_stream.lock().unwrap();
        *playback_stream = None;
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
