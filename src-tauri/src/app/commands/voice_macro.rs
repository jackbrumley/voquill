use serde::Serialize;
use tauri::{command, AppHandle, State};

use crate::config::{MacroStep, VoiceMacroCommand};
use crate::AppState;

#[derive(Clone, Debug, Serialize)]
pub struct SpokenMacroTestResult {
    pub transcript: String,
    pub similarity: f32,
    pub matched: bool,
    pub matched_phrase: Option<String>,
    pub matched_command: Option<VoiceMacroCommand>,
}

#[command]
pub async fn test_voice_macro_sound(state: State<'_, AppState>) -> Result<(), String> {
    let playback_device = {
        let config = state.config.lock().unwrap();
        config.playback_device.clone()
    };
    crate::voice_macro::play_macro_trigger_sound(playback_device);
    Ok(())
}

#[command]
pub async fn test_voice_macro_execution(
    app_handle: AppHandle,
    steps: Vec<MacroStep>,
) -> Result<(), String> {
    crate::voice_macro::execute_macro_steps(&app_handle, "Test Macro", &steps).await
}

#[command]
pub async fn test_spoken_voice_macro(
    _app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<SpokenMacroTestResult, String> {
    let (sample_rate, rx) = {
        let (tx, rx) = std::sync::mpsc::sync_channel::<f32>(65536);
        let mut audio_guard = state.audio_engine.lock().unwrap();
        let engine = audio_guard
            .as_mut()
            .ok_or("Audio engine not initialized. Please check microphone settings.")?;
        *engine.recording_tx.lock().unwrap() = Some(tx);
        (engine.sample_rate, rx)
    };

    let threshold = {
        let config = state.config.lock().unwrap();
        config.voice_macro_activation_threshold
    };

    let (data_tx, data_rx) = std::sync::mpsc::channel::<Vec<f32>>();
    std::thread::spawn(move || {
        let mut samples = Vec::new();
        let mut vad = crate::audio::vad::VoiceActivityDetector::new();
        let frame_size = (sample_rate as f32 * 0.03) as usize;
        let mut frame = Vec::with_capacity(frame_size);
        let max_samples = (sample_rate as f32 * 3.5) as usize;

        let start_time = std::time::Instant::now();
        while start_time.elapsed() < std::time::Duration::from_millis(3500) {
            match rx.recv_timeout(std::time::Duration::from_millis(30)) {
                Ok(s) => {
                    frame.push(s);
                    if frame.len() >= frame_size {
                        let res = vad.process_frame(&frame, threshold);
                        if res.speech_just_ended || samples.len() >= max_samples {
                            samples.extend_from_slice(&frame);
                            break;
                        }
                        if res.is_speaking || vad.smoothed_rms >= threshold * 0.5 {
                            samples.extend_from_slice(&frame);
                        }
                        frame.clear();
                    }
                }
                Err(_) => break,
            }
        }
        let _ = data_tx.send(samples);
    });

    let raw_samples = data_rx.recv().map_err(|e| e.to_string())?;

    {
        if let Some(engine) = state.audio_engine.lock().unwrap().as_ref() {
            *engine.recording_tx.lock().unwrap() = None;
        }
    }

    if raw_samples.is_empty() {
        return Ok(SpokenMacroTestResult {
            transcript: String::new(),
            similarity: 0.0,
            matched: false,
            matched_phrase: None,
            matched_command: None,
        });
    }

    let wav_bytes =
        crate::audio::conversion::finalize_captured_audio_for_whisper(&raw_samples, sample_rate)
            .map_err(|e| e.to_string())?;

    let config_snapshot = {
        let config = state.config.lock().unwrap();
        config.clone()
    };

    let service = state
        .engine_factory
        .create_service(&config_snapshot)
        .await
        .map_err(|e| e.to_string())?;

    let lang = if config_snapshot.language == "auto" {
        None
    } else {
        Some(config_snapshot.language.as_str())
    };

    let prompt = config_snapshot.resolve_prompt_hint();
    let transcript = service
        .transcribe(&wav_bytes, lang, prompt.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    let trimmed = transcript.trim();
    let match_res = crate::voice_macro::find_best_match(
        trimmed,
        &config_snapshot.voice_macro_trigger_word,
        &config_snapshot.voice_macros,
    );

    let matched_phrase = match_res.matched_command.as_ref().map(|c| c.phrase.clone());

    Ok(SpokenMacroTestResult {
        transcript: trimmed.to_string(),
        similarity: match_res.similarity,
        matched: match_res.matched,
        matched_phrase,
        matched_command: match_res.matched_command,
    })
}

#[command]
pub async fn get_available_tts_voices(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<crate::python_runner::VoicePersonaInfo>, String> {
    let runner = state.get_or_start_python_runner(&app_handle).await?;
    runner.get_tts_voices().await
}

#[command]
pub async fn download_tts_voice_model(
    app_handle: AppHandle,
    voice_or_model_id: String,
) -> Result<(), String> {
    crate::python_runner::download_tts_model(&app_handle, &voice_or_model_id).await?;
    Ok(())
}

#[command]
#[allow(clippy::too_many_arguments)]
pub async fn preview_tts_voice(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    text: String,
    voice_id: String,
    speed: f32,
    effect: Option<String>,
    pitch: Option<f32>,
) -> Result<crate::python_runner::TtsSynthesizeResponse, String> {
    if !crate::python_runner::is_tts_model_downloaded(&voice_id) {
        crate::python_runner::download_tts_model(&app_handle, &voice_id).await?;
    }

    let runner = state.get_or_start_python_runner(&app_handle).await?;
    let res = runner
        .synthesize_tts(&text, &voice_id, speed, effect.as_deref(), pitch, None)
        .await?;

    let playback_device = {
        let config = state.config.lock().unwrap();
        config.playback_device.clone()
    };

    let path = std::path::PathBuf::from(&res.output_path);
    if path.exists() {
        let _ = crate::voice_macro::play_macro_sound_file(&path, playback_device);
    }

    Ok(res)
}

#[command]
#[allow(clippy::too_many_arguments)]
pub async fn save_macro_tts_audio(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    macro_id: String,
    text: String,
    voice_id: String,
    speed: f32,
    effect: Option<String>,
    pitch: Option<f32>,
) -> Result<String, String> {
    let dest_path = crate::voice_macro::macro_sound_path(&macro_id)?;
    let dest_str = dest_path.to_string_lossy().to_string();

    if !crate::python_runner::is_tts_model_downloaded(&voice_id) {
        crate::python_runner::download_tts_model(&app_handle, &voice_id).await?;
    }

    let runner = state.get_or_start_python_runner(&app_handle).await?;
    let res = runner
        .synthesize_tts(
            &text,
            &voice_id,
            speed,
            effect.as_deref(),
            pitch,
            Some(&dest_str),
        )
        .await?;

    Ok(res.output_path)
}

#[command]
pub async fn import_macro_audio_file(
    macro_id: String,
    source_path: String,
) -> Result<String, String> {
    crate::voice_macro::import_macro_audio_file(&macro_id, &source_path)
}

#[command]
pub async fn save_macro_mic_recording(
    macro_id: String,
    samples: Vec<f32>,
    sample_rate: u32,
) -> Result<String, String> {
    crate::voice_macro::save_macro_mic_recording(&macro_id, &samples, sample_rate)
}

#[command]
pub async fn play_macro_sound_preview(
    state: State<'_, AppState>,
    macro_id: String,
) -> Result<(), String> {
    let playback_device = {
        let config = state.config.lock().unwrap();
        config.playback_device.clone()
    };

    let path = crate::voice_macro::macro_sound_path(&macro_id)?;
    if path.exists() {
        crate::voice_macro::play_macro_sound_file(&path, playback_device)
    } else {
        crate::voice_macro::play_macro_trigger_sound(playback_device);
        Ok(())
    }
}

#[command]
pub async fn delete_macro_sound(macro_id: String) -> Result<(), String> {
    crate::voice_macro::delete_macro_sound(&macro_id)
}

#[command]
pub async fn clone_macro_sound(
    source_macro_id: String,
    target_macro_id: String,
) -> Result<(), String> {
    crate::voice_macro::clone_macro_sound(&source_macro_id, &target_macro_id)
}

#[command]
pub async fn stop_macro_sound_playback() -> Result<(), String> {
    crate::voice_macro::sound::stop_macro_sound_playback();
    Ok(())
}

#[command]
pub async fn get_available_base_voice_models(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<crate::python_runner::BaseVoiceModelInfo>, String> {
    let runner = state.get_or_start_python_runner(&app_handle).await?;
    runner.get_tts_models().await
}

#[command]
pub async fn preview_custom_tts_voice(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    params: crate::python_runner::CustomTtsSynthesizeParams,
) -> Result<crate::python_runner::TtsSynthesizeResponse, String> {
    crate::log_info!(
        "Preview custom TTS: model={}, speed={:.2}, pitch={:.1}, sub_bass={:.2}, comb={:.2}, flanger={:.2}, bandpass={}, drive={:.2}, rf={:.2}, open='{}', close='{}', text='{}'",
        params.model_key,
        params.speed,
        params.pitch,
        params.sub_bass,
        params.comb_mix,
        params.flanger_mix,
        params.radio_bandpass,
        params.radio_drive,
        params.rf_noise,
        params.opening_chime,
        params.closing_chime,
        params.text
    );
    if !crate::python_runner::is_tts_model_downloaded(&params.model_key) {
        crate::python_runner::download_tts_model(&app_handle, &params.model_key).await?;
    }

    let runner = state.get_or_start_python_runner(&app_handle).await?;
    let res = runner.synthesize_custom_tts(&params).await.map_err(|e| {
        crate::log_warn!("Python runner synthesize_custom_tts failed: {}", e);
        e
    })?;

    let playback_device = {
        let config = state.config.lock().unwrap();
        config.playback_device.clone()
    };

    let path = std::path::PathBuf::from(&res.output_path);
    if path.exists() {
        crate::log_info!(
            "Playing generated custom TTS audio file: {}",
            path.display()
        );
        crate::voice_macro::play_macro_sound_file(&path, playback_device).map_err(|e| {
            crate::log_warn!("Failed to play custom TTS audio file: {}", e);
            e
        })?;
    } else {
        let err_msg = format!(
            "Synthesized audio output file not found on disk: {}",
            path.display()
        );
        crate::log_warn!("{}", err_msg);
        return Err(err_msg);
    }

    Ok(res)
}

#[command]
pub async fn get_custom_voice_presets() -> Result<Vec<serde_json::Value>, String> {
    let presets_file = crate::paths::voice_presets_file()?;
    if !presets_file.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(&presets_file)
        .map_err(|e| format!("Failed to read voice presets: {}", e))?;
    let presets: Vec<serde_json::Value> = serde_json::from_str(&data).unwrap_or_default();
    Ok(presets)
}

#[command]
pub async fn save_custom_voice_preset(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    preset: serde_json::Value,
) -> Result<usize, String> {
    let presets_file = crate::paths::voice_presets_file()?;
    let mut presets: Vec<serde_json::Value> = if presets_file.exists() {
        let data = std::fs::read_to_string(&presets_file).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    };

    let id = preset
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let speed = preset
        .get("speed")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or(1.0);

    let pitch = preset
        .get("pitch")
        .and_then(|v| v.as_f64())
        .map(|v| v as f32)
        .unwrap_or(0.0);

    if !id.is_empty() {
        presets.retain(|p| p.get("id").and_then(|v| v.as_str()) != Some(&id));
    }
    presets.push(preset);

    let json = serde_json::to_string_pretty(&presets)
        .map_err(|e| format!("Failed to serialize voice presets: {}", e))?;
    std::fs::write(&presets_file, json)
        .map_err(|e| format!("Failed to save voice presets: {}", e))?;

    // Automatically regenerate audio on disk for all macros that use this preset
    let mut regenerated_count = 0;
    if !id.is_empty() {
        let matching_macros: Vec<(String, String)> = {
            let config = state.config.lock().unwrap();
            config
                .voice_macros
                .iter()
                .filter(|m| {
                    m.sound_mode == crate::config::MacroSoundMode::Tts
                        && m.sound_tts_voice.as_deref() == Some(&id)
                        && m.sound_tts_text
                            .as_deref()
                            .map(|t| !t.trim().is_empty())
                            .unwrap_or(false)
                })
                .map(|m| (m.id.clone(), m.sound_tts_text.clone().unwrap()))
                .collect()
        };

        for (macro_id, text) in matching_macros {
            crate::log_info!(
                "[Voice Preset] Regenerating audio for macro '{}' using updated preset '{}'...",
                macro_id,
                id
            );
            match save_macro_tts_audio(
                app_handle.clone(),
                state.clone(),
                macro_id.clone(),
                text,
                id.clone(),
                speed,
                Some("custom".to_string()),
                Some(pitch),
            )
            .await
            {
                Ok(_) => {
                    regenerated_count += 1;
                }
                Err(e) => {
                    crate::log_warn!(
                        "[Voice Preset] Failed to regenerate audio for macro '{}': {}",
                        macro_id,
                        e
                    );
                }
            }
        }
    }

    Ok(regenerated_count)
}

#[command]
pub async fn delete_custom_voice_preset(preset_id: String) -> Result<(), String> {
    let presets_file = crate::paths::voice_presets_file()?;
    if !presets_file.exists() {
        return Ok(());
    }
    let data = std::fs::read_to_string(&presets_file).unwrap_or_default();
    let mut presets: Vec<serde_json::Value> = serde_json::from_str(&data).unwrap_or_default();
    presets.retain(|p| p.get("id").and_then(|v| v.as_str()) != Some(&preset_id));

    let json = serde_json::to_string_pretty(&presets)
        .map_err(|e| format!("Failed to serialize voice presets: {}", e))?;
    std::fs::write(&presets_file, json)
        .map_err(|e| format!("Failed to update voice presets: {}", e))?;

    Ok(())
}
