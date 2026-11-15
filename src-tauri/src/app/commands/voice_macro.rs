use serde::Serialize;
use tauri::{command, AppHandle, State};

use crate::config::voice_macro::{MacroStep, VoiceMacroCommand};
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
    crate::voice_macro::play_macro_trigger_sound(&state);
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
pub async fn get_available_tts_voices() -> Result<Vec<crate::tts::VoicePersonaInfo>, String> {
    crate::tts::list_voices()
}

#[command]
pub async fn preview_tts_voice(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    text: String,
    voice_id: String,
    speed: f32,
    effect: Option<String>,
    pitch: Option<f32>,
) -> Result<crate::python_runner::TtsSynthesizeResponse, String> {
    let recipe =
        crate::tts::recipe_for_voice(&voice_id, speed, effect.as_deref(), pitch.unwrap_or(0.0))?;
    let res = crate::tts::render(&app_handle, recipe, &text, None).await?;
    play_rendered(&state, &res)?;
    Ok(res)
}

#[command]
#[allow(clippy::too_many_arguments)]
pub async fn save_macro_tts_audio(
    app_handle: AppHandle,
    macro_id: String,
    text: String,
    voice_id: String,
    speed: f32,
    effect: Option<String>,
    pitch: Option<f32>,
) -> Result<String, String> {
    let dest_path = crate::voice_macro::macro_sound_path(&macro_id)?;
    let recipe =
        crate::tts::recipe_for_voice(&voice_id, speed, effect.as_deref(), pitch.unwrap_or(0.0))?;
    let res = crate::tts::render(
        &app_handle,
        recipe,
        &text,
        Some(dest_path.to_string_lossy().into_owned()),
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
    let path = crate::voice_macro::macro_sound_path(&macro_id)?;
    if path.exists() {
        crate::voice_macro::play_macro_sound_file(&state, &path)
    } else {
        crate::voice_macro::play_macro_trigger_sound(&state);
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
pub async fn stop_macro_sound_playback(state: State<'_, AppState>) -> Result<(), String> {
    crate::voice_macro::sound::stop_macro_sound_playback(&state);
    Ok(())
}

#[command]
pub async fn get_available_base_voice_models(
) -> Result<Vec<crate::python_runner::tts_models::BaseVoiceModelInfo>, String> {
    Ok(crate::python_runner::tts_models::base_voice_models())
}

/// A Voice Studio preview: text plus the tuning being designed.
#[derive(serde::Deserialize)]
pub struct VoiceStudioPreview {
    pub text: String,
    #[serde(flatten)]
    pub tuning: crate::tts::presets::VoiceTuning,
}

#[command]
pub async fn preview_custom_tts_voice(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    params: VoiceStudioPreview,
) -> Result<crate::python_runner::TtsSynthesizeResponse, String> {
    crate::log_info!(
        "Voice Studio preview: model={}, speed={:.2}, pitch={:.1}",
        params.tuning.model_key,
        params.tuning.speed,
        params.tuning.pitch
    );
    let recipe = crate::tts::recipe_for_tuning(&params.tuning);
    let res = crate::tts::render(&app_handle, recipe, &params.text, None).await?;
    play_rendered(&state, &res)?;
    Ok(res)
}

/// Plays a freshly rendered clip, failing if Python wrote no file.
fn play_rendered(
    state: &AppState,
    rendered: &crate::python_runner::TtsSynthesizeResponse,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&rendered.output_path);
    if !path.exists() {
        return Err(format!(
            "Synthesized audio was not written to {}",
            path.display()
        ));
    }
    crate::voice_macro::play_macro_sound_file(state, &path)
}

#[command]
pub async fn get_custom_voice_presets() -> Result<Vec<crate::tts::presets::VoicePreset>, String> {
    crate::tts::presets::load()
}

/// Saves a preset and re-renders the spoken feedback of every macro that
/// uses it, returning how many were updated.
#[command]
pub async fn save_custom_voice_preset(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    preset: crate::tts::presets::VoicePreset,
) -> Result<usize, String> {
    let preset_id = preset.id.clone();
    let recipe = crate::tts::recipe_for_tuning(&preset.tuning);
    crate::tts::presets::save(preset)?;

    let macros_using_preset: Vec<(String, String)> = {
        let config = state.config.lock().unwrap();
        config
            .voice_macros
            .iter()
            .filter(|m| {
                m.sound_mode == crate::config::voice_macro::MacroSoundMode::Tts
                    && m.sound_tts_voice.as_deref() == Some(preset_id.as_str())
            })
            .filter_map(|m| {
                let text = m.sound_tts_text.as_deref()?.trim();
                (!text.is_empty()).then(|| (m.id.clone(), text.to_string()))
            })
            .collect()
    };

    let mut regenerated_count = 0;
    for (macro_id, text) in macros_using_preset {
        crate::log_info!(
            "[Voice Preset] Regenerating audio for macro '{}' with preset '{}'",
            macro_id,
            preset_id
        );
        let output = crate::voice_macro::macro_sound_path(&macro_id)?
            .to_string_lossy()
            .into_owned();
        match crate::tts::render(&app_handle, recipe.clone(), &text, Some(output)).await {
            Ok(_) => regenerated_count += 1,
            Err(e) => crate::log_warn!(
                "[Voice Preset] Failed to regenerate audio for macro '{}': {}",
                macro_id,
                e
            ),
        }
    }
    Ok(regenerated_count)
}

#[command]
pub async fn delete_custom_voice_preset(preset_id: String) -> Result<(), String> {
    crate::tts::presets::delete(&preset_id)
}
