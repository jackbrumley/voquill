use crate::{audio, history};
use tauri::{Emitter, Manager};

#[tauri::command]
pub async fn transcribe_audio_file(
    path: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    crate::log_info!("transcribe_audio_file: {}", path);

    let audio_data = std::fs::read(&path).map_err(|e| format!("Failed to read file: {}", e))?;

    let wav_data = audio::convert_audio_file_for_whisper(&audio_data)
        .map_err(|e| format!("Failed to convert audio: {}", e))?;

    let app_state = app_handle.state::<crate::AppState>();
    let current_config = {
        let guard = app_state.config.lock().unwrap();
        guard.clone()
    };
    let engine_factory_state = app_state.engine_factory.clone();
    let service = engine_factory_state
        .create_service(&current_config)
        .await
        .map_err(|e| format!("Failed to create transcription service: {}", e))?;

    let language = current_config.language.clone();
    let (lang_code, prompt_hint) = match language.as_str() {
        "auto" => (None, None),
        "en-AU" => (Some("en"), Some("Australian spelling.")),
        "en-GB" => (Some("en"), Some("British spelling.")),
        "en-US" => (Some("en"), Some("American spelling.")),
        code => (Some(code), None),
    };

    let text = match service.transcribe(&wav_data, lang_code, prompt_hint).await {
        Ok(t) => t,
        Err(e) => return Err(format!("Transcription failed: {}", e)),
    };

    if !text.trim().is_empty() {
        if let Err(e) = history::add_history_item(&text) {
            crate::log_warn!("Failed to save history: {}", e);
        }
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("history-updated", ());
        }
    }

    Ok(text)
}
