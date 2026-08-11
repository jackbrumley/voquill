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
    let dictionary_words = current_config.dictionary.clone();
    let lang_code = match language.as_str() {
        "auto" => None,
        "en-AU" => Some("en"),
        "en-GB" => Some("en"),
        "en-US" => Some("en"),
        code => Some(code),
    };

    let mut prompt_hint: Option<String> = match language.as_str() {
        "en-AU" => Some("Australian spelling.".to_string()),
        "en-GB" => Some("British spelling.".to_string()),
        "en-US" => Some("American spelling.".to_string()),
        _ => None,
    };

    if !dictionary_words.is_empty() {
        let dict_str = dictionary_words.join(", ");
        prompt_hint = match prompt_hint {
            Some(hint) => Some(format!("{}, {}", hint, dict_str)),
            None => Some(dict_str),
        };
    }

    let text = match service
        .transcribe(&wav_data, lang_code, prompt_hint.as_deref())
        .await
    {
        Ok(text) => {
            crate::log_info!(
                "Transcription received ({}): \"{}\"",
                service.service_name(),
                text
            );
            text
        }
        Err(e) => return Err(format!("Transcription failed: {}", e)),
    };

    let text = if !text.trim().is_empty() && current_config.post_process_enabled {
        crate::log_info!("Post-processing file transcription...");
        let post_process_factory = app_state.post_process_factory.clone();
        match post_process_factory.get_service(&current_config).await {
            Ok(processor) => match processor
                .post_process(&text, &current_config.post_process_prompt)
                .await
            {
                Ok(cleaned) => {
                    crate::log_info!(
                        "Post-processed ({}): \"{}\"",
                        processor.service_name(),
                        cleaned
                    );
                    cleaned
                }
                Err(e) => {
                    crate::log_warn!("Post-processing failed, using raw text: {}", e);
                    if matches!(e, crate::post_process::PostProcessError::Network(_)) {
                        post_process_factory.invalidate_local();
                    }
                    text
                }
            },
            Err(e) => {
                crate::log_warn!(
                    "Could not create post-process service, using raw text: {}",
                    e
                );
                text
            }
        }
    } else {
        text
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
