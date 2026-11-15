//! Turns a spoken phrase into a transcript and the macro it matches. Shared
//! by the background listener and the spoken macro tester, so a test result
//! is exactly what the listener would have done with the same audio.

use super::matcher::{find_best_match, MacroMatchResult};
use crate::audio::conversion::finalize_captured_audio_for_whisper;
use crate::AppState;

pub struct Recognition {
    pub transcript: String,
    pub match_result: MacroMatchResult,
}

pub async fn recognize_phrase(
    state: &AppState,
    samples: &[f32],
    sample_rate: u32,
) -> Result<Recognition, String> {
    let wav_bytes = finalize_captured_audio_for_whisper(samples, sample_rate)
        .map_err(|e| format!("Audio conversion failed: {}", e))?;
    let config = state.config.lock().unwrap().clone();
    let service = state
        .engine_factory
        .create_service(&config)
        .await
        .map_err(|e| format!("Transcription service unavailable: {}", e))?;
    let language = (config.language != "auto").then_some(config.language.as_str());
    let prompt_hint = config.resolve_prompt_hint();
    let transcript = service
        .transcribe(&wav_bytes, language, prompt_hint.as_deref())
        .await
        .map_err(|e| format!("Transcription failed: {}", e))?
        .trim()
        .to_string();
    let match_result = find_best_match(
        &transcript,
        &config.voice_macro_trigger_word,
        &config.voice_macros,
    );
    Ok(Recognition {
        transcript,
        match_result,
    })
}
