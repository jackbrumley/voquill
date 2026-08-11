use crate::engine_factory;
use crate::{history, model_manager, transcription};
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
) -> Result<(), String> {
    let model = model_manager::ModelManager::find_model(&engine_name, &model_size)
        .ok_or_else(|| format!("Model {} not found for engine {}", model_size, engine_name))?;
    let manager = model_manager::ModelManager::new().map_err(|error| error.to_string())?;

    manager
        .download_model(&model, move |progress: model_manager::DownloadProgress| {
            let _ = app_handle.emit("model-download-progress", progress);
        })
        .await?;

    Ok(())
}

#[tauri::command]
pub fn get_current_status() -> String {
    crate::app::status::get_current_status()
}

#[tauri::command]
pub async fn get_history() -> Result<Vec<history::HistoryItem>, String> {
    history::load_history().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn search_history(query: String) -> Result<Vec<history::HistoryItem>, String> {
    history::search_history(&query).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clear_history() -> Result<(), String> {
    history::clear_history().map_err(|error| error.to_string())
}
