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
