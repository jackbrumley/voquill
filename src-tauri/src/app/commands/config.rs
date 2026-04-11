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
    normalized_config.normalize_input_sensitivity();

    let (restart_engine, hotkey_changed) = {
        let mut config_guard = state.config.lock().unwrap();
        let audio_changed = config_guard.audio_device != normalized_config.audio_device
            || config_guard.input_sensitivity != normalized_config.input_sensitivity;
        let hotkey_changed = config_guard.hotkey != normalized_config.hotkey;

        let mut merged_config = normalized_config.clone();
        if merged_config.shortcuts_token.is_none() {
            merged_config.shortcuts_token = config_guard.shortcuts_token.clone();
        }
        if merged_config.input_token.is_none() {
            merged_config.input_token = config_guard.input_token.clone();
        }

        *config_guard = merged_config;

        let mut cached_device = state.cached_device.lock().unwrap();
        *cached_device = audio::lookup_device(config_guard.audio_device.clone()).ok();
        crate::log_info!("🔧 Pre-warmed audio device cache");

        (audio_changed, hotkey_changed)
    };

    let is_mic_test_active = *state.is_mic_test_active.lock().unwrap();
    if restart_engine && !is_mic_test_active {
        crate::log_info!("🔧 Audio config changed, restarting persistent engine...");
        let cached_device = state.cached_device.lock().unwrap().clone();
        let sensitivity = normalized_config.input_sensitivity;
        let mut engine_guard = state.audio_engine.lock().unwrap();
        *engine_guard = None;
        if let Some(device) = cached_device {
            if let Ok(new_engine) = audio::PersistentAudioEngine::new(&device, sensitivity) {
                *engine_guard = Some(new_engine);
                crate::log_info!("✅ Persistent engine restarted");
            }
        }
    } else if restart_engine {
        crate::log_info!(
            "🔧 Audio config changed during active mic test, deferring engine restart"
        );
    }

    if let Err(error) = config::save_config(&normalized_config) {
        return Err(format!("Failed to save config: {}", error));
    }

    if hotkey_changed {
        if let Err(error) = re_register_hotkey(&app_handle, &normalized_config.hotkey).await {
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

    Ok(())
}

#[tauri::command]
pub async fn reset_application_to_defaults(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::log_info!("🧹 Factory reset requested");

    let root_dir = crate::get_app_config_root_dir()?;

    let models_dir = root_dir.join("models");
    if models_dir.exists() {
        std::fs::remove_dir_all(&models_dir).map_err(|error| error.to_string())?;
    }
    std::fs::create_dir_all(&models_dir).map_err(|error| error.to_string())?;

    let debug_dir = root_dir.join("debug");
    std::fs::create_dir_all(&debug_dir).map_err(|error| error.to_string())?;
    crate::clear_directory_contents(&debug_dir, &["session.log"])?;

    if let Err(error) = crate::truncate_session_log_with_header() {
        crate::log_warn!(
            "⚠️ Could not truncate session log during factory reset: {}",
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

    crate::log_info!("✅ Factory reset completed successfully");
    Ok(())
}
