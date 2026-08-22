use crate::engine_factory;
use crate::{history, model_manager, transcription, AppState};
use tauri::Emitter;

#[tauri::command]
pub async fn test_api_key(api_key: String, api_url: String) -> Result<bool, String> {
    transcription::test_api_key(&api_key, &api_url)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_available_engines() -> Result<Vec<String>, String> {
    Ok(model_manager::ModelManager::get_available_engines())
}

#[tauri::command]
pub async fn get_available_models() -> Result<Vec<model_manager::ModelInfo>, String> {
    Ok(model_manager::ModelManager::get_available_models())
}

#[tauri::command]
pub fn get_engine_capabilities(engine_name: String) -> engine_factory::EngineCapabilities {
    engine_factory::EngineFactory::engine_capabilities(&engine_name)
}

#[tauri::command]
pub async fn check_model_status(model_size: String, engine_name: String) -> Result<bool, String> {
    let model = model_manager::ModelManager::find_model(&engine_name, &model_size)
        .ok_or_else(|| format!("Model {} not found for engine {}", model_size, engine_name))?;
    let manager = model_manager::ModelManager::new().map_err(|error| error.to_string())?;
    Ok(manager.is_model_downloaded(&model))
}

#[tauri::command]
pub async fn download_model(
    model_size: String,
    engine_name: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let model = model_manager::ModelManager::find_model(&engine_name, &model_size)
        .ok_or_else(|| format!("Model {} not found for engine {}", model_size, engine_name))?;
    let manager = model_manager::ModelManager::new().map_err(|error| error.to_string())?;

    let progress_handle = app_handle.clone();
    manager
        .download_model(&model, move |progress: model_manager::DownloadProgress| {
            let _ = progress_handle.emit("model-download-progress", progress);
        })
        .await?;

    // A completed download may satisfy a warm-up that startup skipped
    // because the model was missing. Re-arm whichever engine this model
    // belongs to so the first dictation reuses a warm service.
    let config = state.config.lock().unwrap().clone();

    if config.post_process_enabled
        && config.post_process_provider == crate::config::PostProcessProvider::Local
        && config.post_process_engine == engine_name
        && config.post_process_model == model_size
    {
        crate::app::bootstrap::spawn_post_process_warmup(
            state.post_process_factory.clone(),
            &config,
            &app_handle,
        );
    }

    if config.transcription_mode == crate::config::TranscriptionMode::Local
        && config.local_engine == engine_name
        && config.local_model_size == model_size
    {
        crate::app::bootstrap::spawn_engine_preload(state.engine_factory.clone(), &config);
    }

    Ok(())
}

/// Loads the configured transcription model into the engine cache and awaits
/// completion. Unlike the fire-and-forget startup/download preloads, this lets
/// the frontend deterministically verify engine loading (e.g. during initial
/// setup): a GPU engine either loads on GPU or records the fallback reason,
/// which `get_gpu_status` then reports.
#[tauri::command]
pub async fn preload_transcription_engine(state: tauri::State<'_, AppState>) -> Result<(), String> {
    crate::log_info!("Tauri Command: preload_transcription_engine invoked");
    let config = { state.config.lock().unwrap().clone() };
    state.engine_factory.preload(&config).await;
    Ok(())
}

#[tauri::command]
pub fn get_current_status() -> String {
    crate::app::status::get_current_status()
}

#[tauri::command]
pub async fn get_history(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<history::HistoryItem>, String> {
    let limit = state.config.lock().unwrap().history_limit;
    history::load_history(limit).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn search_history(
    query: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<history::HistoryItem>, String> {
    let limit = state.config.lock().unwrap().history_limit;
    history::search_history(&query, limit).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clear_history() -> Result<(), String> {
    history::clear_history().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn delete_history_item(id: u64) -> Result<(), String> {
    history::delete_history_item(id).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_history_audio(file_name: String) -> Result<Vec<u8>, String> {
    let recordings_dir = crate::paths::debug_recordings_dir()?;
    let path = recordings_dir.join(&file_name);
    if path.exists() {
        return std::fs::read(&path).map_err(|e| e.to_string());
    }
    let debug_dir = crate::paths::debug_dir()?;
    let alt_path = debug_dir.join(&file_name);
    if alt_path.exists() {
        return std::fs::read(&alt_path).map_err(|e| e.to_string());
    }
    Err("Audio file not found".to_string())
}

#[tauri::command]
pub async fn unload_model(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.engine_factory.unload_all();
    crate::log_info!("Transcription model unloaded");
    Ok(())
}
