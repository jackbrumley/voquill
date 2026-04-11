use crate::config::{self, Config, TranscriptionMode};
use crate::{audio, history, local_whisper, transcription, typing};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

fn validate_audio_duration(
    audio_data: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if audio_data.len() < 44 {
        return Err("Audio file too small".into());
    }
    let sample_rate = u32::from_le_bytes([
        audio_data[24],
        audio_data[25],
        audio_data[26],
        audio_data[27],
    ]);
    let channels = u16::from_le_bytes([audio_data[22], audio_data[23]]);
    let bits_per_sample = u16::from_le_bytes([audio_data[34], audio_data[35]]);

    let mut data_size = 0u32;
    let mut pos = 36;
    while pos + 8 <= audio_data.len() {
        let chunk_id = &audio_data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            audio_data[pos + 4],
            audio_data[pos + 5],
            audio_data[pos + 6],
            audio_data[pos + 7],
        ]);
        if chunk_id == b"data" {
            data_size = chunk_size;
            break;
        }
        pos += 8 + chunk_size as usize;
        if chunk_size % 2 == 1 {
            pos += 1;
        }
    }

    if data_size == 0 {
        return Err("No data chunk".into());
    }
    let bytes_per_sample = (bits_per_sample / 8) as u32;
    let bytes_per_second = sample_rate * channels as u32 * bytes_per_sample;
    let duration_seconds = data_size as f64 / bytes_per_second as f64;

    crate::log_info!("Audio duration: {:.3}s", duration_seconds);
    if duration_seconds < 0.1 {
        return Err("Audio too short".into());
    }
    Ok(())
}

pub async fn record_and_transcribe(
    config: Arc<Mutex<Config>>,
    is_recording: Arc<Mutex<bool>>,
    app_handle: AppHandle,
    audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reset_status_on_exit = || async {
        crate::app::status::emit_status_to_frontend("Ready").await;
    };

    let audio_data = match audio::record_audio_while_flag(&is_recording, audio_engine).await {
        Ok(data) => data,
        Err(error) => {
            reset_status_on_exit().await;
            return Err(error);
        }
    };

    if audio_data.is_empty() {
        reset_status_on_exit().await;
        return Ok(());
    }
    if let Err(error) = validate_audio_duration(&audio_data) {
        crate::log_info!("⚠️ Audio validation failed: {}", error);
        reset_status_on_exit().await;
        return Ok(());
    }

    crate::app::status::emit_status_to_frontend("Transcribing").await;
    let (
        transcription_mode,
        api_key,
        api_url,
        api_model,
        debug_mode,
        enable_recording_logs,
        language_choice,
    ) = {
        let config_guard = config.lock().unwrap();
        (
            config_guard.transcription_mode.clone(),
            config_guard.openai_api_key.clone(),
            config_guard.api_url.clone(),
            config_guard.api_model.clone(),
            config_guard.debug_mode,
            config_guard.enable_recording_logs,
            config_guard.language.clone(),
        )
    };

    let (lang_code, prompt_hint) = match language_choice.as_str() {
        "auto" => (None, None),
        "en-AU" => (Some("en"), Some("Australian spelling.")),
        "en-GB" => (Some("en"), Some("British spelling.")),
        "en-US" => (Some("en"), Some("American spelling.")),
        code => (Some(code), None),
    };

    if debug_mode && enable_recording_logs {
        let debug_path = dirs::config_dir()
            .unwrap_or_default()
            .join("foss-voquill")
            .join("debug")
            .join(format!(
                "recording_{}.wav",
                ::chrono::Local::now().format("%Y%m%d_%H%M%S")
            ));

        if let Err(error) = std::fs::create_dir_all(debug_path.parent().unwrap()) {
            crate::log_info!("❌ Failed to create debug directory: {}", error);
        } else if let Err(error) = std::fs::write(&debug_path, &audio_data) {
            crate::log_info!("❌ Failed to save debug recording: {}", error);
        } else {
            crate::log_info!("🛡️ Debug recording saved to: {:?}", debug_path);
        }
    }

    crate::log_info!("📡 Transcription Mode: {:?}", transcription_mode);
    crate::log_info!("🌐 Language: {:?}, Hint: {:?}", lang_code, prompt_hint);

    let service: Box<dyn transcription::TranscriptionService + Send + Sync> =
        match transcription_mode {
            TranscriptionMode::API => Box::new(transcription::APITranscriptionService {
                api_key,
                api_url,
                api_model,
            }),
            TranscriptionMode::Local => {
                let (model_size, use_gpu) = {
                    let config_lock = config.lock().unwrap();
                    (config_lock.local_model_size.clone(), config_lock.enable_gpu)
                };
                match local_whisper::LocalWhisperService::new(&model_size, use_gpu) {
                    Ok(service) => Box::new(service),
                    Err(error) => {
                        crate::log_info!("❌ Failed to initialize Local Whisper: {}", error);
                        reset_status_on_exit().await;
                        return Err(error.into());
                    }
                }
            }
        };

    let text = match service
        .transcribe(&audio_data, lang_code, prompt_hint)
        .await
    {
        Ok(text) => {
            crate::log_info!(
                "📝 Transcription received ({}): \"{}\"",
                service.service_name(),
                text
            );
            text
        }

        Err(error) => {
            crate::log_info!(
                "❌ Transcription failed ({}): {}",
                service.service_name(),
                error
            );
            reset_status_on_exit().await;
            return Err(error.into());
        }
    };

    if !text.trim().is_empty() {
        let _ = history::add_history_item(&text);
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("history-updated", ());
        }

        crate::app::status::emit_status_to_frontend("Typing").await;
        let (typing_speed, hold_duration, output_method, copy_on_typewriter) = {
            let config_guard = config.lock().unwrap();
            (
                config_guard.typing_speed_interval,
                config_guard.key_press_duration_ms,
                config_guard.output_method.clone(),
                config_guard.copy_on_typewriter,
            )
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        match output_method {
            config::OutputMethod::Typewriter => {
                if copy_on_typewriter {
                    if let Err(error) = typing::copy_to_clipboard(&text) {
                        crate::log_info!("❌ CLIPBOARD ERROR: {}", error);
                    }
                }
                crate::log_info!("⌨️  Forwarding text to hardware typing engine...");
                let state = app_handle.state::<crate::AppState>();
                if let Err(error) = state
                    .display_backend
                    .type_text_hardware(&app_handle, &text, typing_speed, hold_duration)
                    .await
                {
                    crate::log_info!("❌ TYPING ENGINE ERROR: {}", error);
                }
            }
            config::OutputMethod::Clipboard => {
                crate::log_info!("📋 Copying text to clipboard (Clipboard Mode)...");
                if let Err(error) = typing::copy_to_clipboard(&text) {
                    crate::log_info!("❌ CLIPBOARD ERROR: {}", error);
                }
            }
        }
    } else {
        crate::log_info!("ℹ️ Transcription was empty, skipping typing.");
    }

    reset_status_on_exit().await;
    Ok(())
}
