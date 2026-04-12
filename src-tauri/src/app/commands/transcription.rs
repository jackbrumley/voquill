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
pub async fn check_model_status(model_size: String) -> Result<bool, String> {
    let manager = model_manager::ModelManager::new().map_err(|error| error.to_string())?;
    Ok(manager.is_model_downloaded(&model_size))
}

#[tauri::command]
pub async fn download_model(model_size: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let manager = model_manager::ModelManager::new().map_err(|error| error.to_string())?;

    manager
        .download_model(&model_size, move |progress| {
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
pub async fn get_history() -> Result<history::History, String> {
    history::load_history().map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clear_history() -> Result<(), String> {
    history::clear_history().map_err(|error| error.to_string())
}
