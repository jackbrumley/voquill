//! Text-to-speech for voice macro feedback. Rust decides everything about a
//! voice: which persona or preset an ID means, which base voice files it
//! uses (downloading them first), and the speaker, speed and sound effects.
//! The Python runner only renders the fully resolved `RenderRequest`.

pub mod personas;
pub mod presets;

use crate::python_runner::tts_models;
use presets::VoiceTuning;
use serde::Serialize;
use tauri::AppHandle;

/// Speeds outside this range sound broken, so every request is clamped.
const SPEED_RANGE: (f32, f32) = (0.5, 2.0);
/// A requested speed or pitch this close to neutral means "use the voice's own".
const NEUTRAL_TOLERANCE: f32 = 0.01;

/// A voice as the voice pickers list it.
#[derive(Clone, Debug, Serialize)]
pub struct VoicePersonaInfo {
    pub id: String,
    pub name: String,
    pub persona: String,
    pub category: String,
    pub engine: String,
    pub description: String,
    pub is_ready: bool,
    pub default_effect: Option<String>,
    pub default_pitch: Option<f32>,
    pub default_speed: Option<f32>,
}

/// The sound processing applied after synthesis.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Dsp {
    /// A named persona effect (radio, mech, eva, ...) with a pitch shift.
    Effect { effect: String, pitch: f32 },
    /// Voice Studio's individual tuning knobs.
    Custom {
        pitch: f32,
        sub_bass: f32,
        comb_mix: f32,
        flanger_mix: f32,
        radio_bandpass: bool,
        radio_drive: f32,
        rf_noise: f32,
        opening_chime: String,
        closing_chime: String,
    },
}

/// Everything needed to render speech except the text and where to write it.
#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    pub base_model: String,
    pub speaker_id: i64,
    pub speed: f32,
    pub noise_scale: f32,
    pub dsp: Dsp,
}

/// What the Python runner receives: concrete files and parameters only.
#[derive(Debug, Serialize)]
pub struct RenderRequest {
    pub text: String,
    pub model_path: String,
    pub tokens_path: String,
    pub data_dir: String,
    pub speaker_id: i64,
    pub speed: f32,
    pub noise_scale: f32,
    pub dsp: Dsp,
    pub output_path: Option<String>,
}

fn clamp_speed(speed: f32) -> f32 {
    speed.clamp(SPEED_RANGE.0, SPEED_RANGE.1)
}

fn requested_or(requested: f32, neutral: f32, own: f32) -> f32 {
    if (requested - neutral).abs() > NEUTRAL_TOLERANCE {
        requested
    } else {
        own
    }
}

/// The recipe for a Voice Studio tuning (a preset or a live preview).
pub fn recipe_for_tuning(tuning: &VoiceTuning) -> Recipe {
    let radio_on = tuning.radio_bandpass;
    Recipe {
        base_model: tuning.model_key.clone(),
        speaker_id: tuning.speaker_id,
        speed: clamp_speed(tuning.speed),
        noise_scale: tuning.noise_scale,
        dsp: Dsp::Custom {
            pitch: tuning.pitch,
            sub_bass: tuning.sub_bass,
            comb_mix: tuning.comb_mix,
            flanger_mix: tuning.flanger_mix,
            radio_bandpass: radio_on,
            radio_drive: if radio_on { tuning.radio_drive } else { 1.0 },
            rf_noise: if radio_on { tuning.rf_noise } else { 0.0 },
            opening_chime: tuning.opening_chime.clone(),
            closing_chime: tuning.closing_chime.clone(),
        },
    }
}

/// The recipe for a voice ID: a saved preset, a persona (or a legacy ID
/// that maps to one), or a base voice by its model key. A speed or pitch
/// other than neutral overrides the voice's own.
pub fn recipe_for_voice(
    voice_id: &str,
    speed: f32,
    effect: Option<&str>,
    pitch: f32,
) -> Result<Recipe, String> {
    if let Some(preset) = presets::find(voice_id)? {
        let mut tuning = preset.tuning;
        tuning.speed = requested_or(speed, 1.0, tuning.speed);
        tuning.pitch = requested_or(pitch, 0.0, tuning.pitch);
        return Ok(recipe_for_tuning(&tuning));
    }
    if voice_id.starts_with("preset-") {
        return Err(format!(
            "Custom voice preset '{}' was not found. Save it in Voice Studio first.",
            voice_id
        ));
    }
    if let Some(persona) = personas::find(voice_id) {
        let chosen_effect = effect
            .filter(|name| !matches!(*name, "default" | "clean"))
            .unwrap_or(persona.default_effect);
        return Ok(Recipe {
            base_model: persona.base_model.to_string(),
            speaker_id: 0,
            speed: clamp_speed(requested_or(speed, 1.0, persona.default_speed)),
            noise_scale: presets::DEFAULT_NOISE_SCALE,
            dsp: Dsp::Effect {
                effect: chosen_effect.to_string(),
                pitch: requested_or(pitch, 0.0, persona.default_pitch),
            },
        });
    }
    if tts_models::voice_files(voice_id).is_ok() {
        return Ok(Recipe {
            base_model: voice_id.to_string(),
            speaker_id: 0,
            speed: clamp_speed(speed),
            noise_scale: presets::DEFAULT_NOISE_SCALE,
            dsp: Dsp::Effect {
                effect: effect.unwrap_or("clean").to_string(),
                pitch,
            },
        });
    }
    Err(format!("Unknown voice '{}'", voice_id))
}

/// Every voice the pickers offer: the personas, then the saved presets.
pub fn list_voices() -> Result<Vec<VoicePersonaInfo>, String> {
    let mut voices: Vec<VoicePersonaInfo> = personas::PERSONAS
        .iter()
        .map(|persona| VoicePersonaInfo {
            id: persona.id.to_string(),
            name: persona.name.to_string(),
            persona: persona.persona.to_string(),
            category: persona.category.to_string(),
            engine: "piper".to_string(),
            description: persona.description.to_string(),
            is_ready: tts_models::is_tts_model_downloaded(persona.base_model),
            default_effect: Some(persona.default_effect.to_string()),
            default_pitch: Some(persona.default_pitch),
            default_speed: Some(persona.default_speed),
        })
        .collect();
    voices.extend(presets::load()?.into_iter().map(|preset| VoicePersonaInfo {
        is_ready: tts_models::is_tts_model_downloaded(&preset.tuning.model_key),
        id: preset.id,
        name: preset.name,
        persona: "Custom User Preset".to_string(),
        category: preset.category,
        engine: "piper".to_string(),
        description: preset.description,
        default_effect: Some("custom".to_string()),
        default_pitch: Some(preset.tuning.pitch),
        default_speed: Some(preset.tuning.speed),
    }));
    Ok(voices)
}

/// Renders `text` with `recipe`, downloading the base voice first if needed.
pub async fn render(
    app_handle: &AppHandle,
    recipe: Recipe,
    text: &str,
    output_path: Option<String>,
) -> Result<crate::python_runner::TtsSynthesizeResponse, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("There is no text to speak".to_string());
    }
    tts_models::download_tts_model(app_handle, &recipe.base_model).await?;
    let files = tts_models::voice_files(&recipe.base_model)?;
    let request = RenderRequest {
        text: text.to_string(),
        model_path: files.model_path.to_string_lossy().into_owned(),
        tokens_path: files.tokens_path.to_string_lossy().into_owned(),
        data_dir: files.data_dir.to_string_lossy().into_owned(),
        speaker_id: recipe.speaker_id,
        speed: recipe.speed,
        noise_scale: recipe.noise_scale,
        dsp: recipe.dsp,
        output_path,
    };
    let state = tauri::Manager::state::<crate::AppState>(app_handle);
    let runner = state.get_or_start_python_runner(app_handle).await?;
    runner.render_tts(&request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persona_defaults_apply_unless_overridden() {
        let recipe = recipe_for_voice("titan-mech", 1.0, None, 0.0).unwrap();
        assert_eq!(recipe.base_model, "piper-en_US-ryan-low");
        assert_eq!(recipe.speed, 0.95);
        assert_eq!(
            recipe.dsp,
            Dsp::Effect {
                effect: "mech".to_string(),
                pitch: -4.0
            }
        );

        let recipe = recipe_for_voice("titan-mech", 1.3, Some("radio"), 2.0).unwrap();
        assert_eq!(recipe.speed, 1.3);
        assert_eq!(
            recipe.dsp,
            Dsp::Effect {
                effect: "radio".to_string(),
                pitch: 2.0
            }
        );
    }

    #[test]
    fn base_voice_ids_render_clean() {
        let recipe = recipe_for_voice("piper-en_GB-alan-medium", 3.0, None, 0.0).unwrap();
        assert_eq!(recipe.base_model, "piper-en_GB-alan-medium");
        assert_eq!(recipe.speed, SPEED_RANGE.1);
    }

    #[test]
    fn unknown_voices_and_missing_presets_fail() {
        assert!(recipe_for_voice("not-a-voice", 1.0, None, 0.0).is_err());
        assert!(recipe_for_voice("preset-missing-for-test", 1.0, None, 0.0).is_err());
    }

    #[test]
    fn radio_settings_only_apply_with_radio_on() {
        let tuning: VoiceTuning = serde_json::from_str(
            r#"{"model_key": "piper-en_US-ryan-low", "radio_bandpass": false, "radio_drive": 3.0, "rf_noise": 0.5}"#,
        )
        .unwrap();
        let Dsp::Custom {
            radio_drive,
            rf_noise,
            ..
        } = recipe_for_tuning(&tuning).dsp
        else {
            panic!("expected custom DSP");
        };
        assert_eq!((radio_drive, rf_noise), (1.0, 0.0));
    }
}
