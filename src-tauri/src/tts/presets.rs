//! Voice Studio presets saved in `voice_presets.json`: the single owner of
//! that file, read and written as typed presets.

use serde::{Deserialize, Serialize};

fn default_speed() -> f32 {
    1.0
}
/// The base voices' standard variation in delivery.
pub const DEFAULT_NOISE_SCALE: f32 = 0.667;

fn default_noise_scale() -> f32 {
    DEFAULT_NOISE_SCALE
}
fn default_radio_drive() -> f32 {
    1.0
}
fn default_chime() -> String {
    "none".to_string()
}
fn default_category() -> String {
    "Custom Presets".to_string()
}

/// How a voice sounds: a base voice and the tuning applied to it. Shared by
/// saved presets and Voice Studio previews. Missing fields (from files
/// written by older versions) take their defaults.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VoiceTuning {
    pub model_key: String,
    #[serde(default)]
    pub speaker_id: i64,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default = "default_noise_scale")]
    pub noise_scale: f32,
    #[serde(default)]
    pub pitch: f32,
    #[serde(default)]
    pub sub_bass: f32,
    #[serde(default)]
    pub comb_mix: f32,
    #[serde(default)]
    pub flanger_mix: f32,
    #[serde(default)]
    pub radio_bandpass: bool,
    #[serde(default = "default_radio_drive")]
    pub radio_drive: f32,
    #[serde(default)]
    pub rf_noise: f32,
    #[serde(default = "default_chime")]
    pub opening_chime: String,
    #[serde(default = "default_chime")]
    pub closing_chime: String,
}

/// One saved voice.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VoicePreset {
    pub id: String,
    pub name: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(flatten)]
    pub tuning: VoiceTuning,
}

/// Every saved preset. A missing file means none; an unreadable one is an
/// error rather than silently empty.
pub fn load() -> Result<Vec<VoicePreset>, String> {
    let path = crate::paths::voice_presets_file()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read voice presets {}: {}", path.display(), e))?;
    serde_json::from_str(&data)
        .map_err(|e| format!("Voice presets file {} is not valid: {}", path.display(), e))
}

pub fn find(preset_id: &str) -> Result<Option<VoicePreset>, String> {
    Ok(load()?.into_iter().find(|preset| preset.id == preset_id))
}

fn write(presets: &[VoicePreset]) -> Result<(), String> {
    let path = crate::paths::voice_presets_file()?;
    let json = serde_json::to_string_pretty(presets)
        .map_err(|e| format!("Failed to serialize voice presets: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to save voice presets: {}", e))
}

/// Adds the preset, replacing one with the same ID.
pub fn save(preset: VoicePreset) -> Result<(), String> {
    let mut presets = load()?;
    presets.retain(|existing| existing.id != preset.id);
    presets.push(preset);
    write(&presets)
}

pub fn delete(preset_id: &str) -> Result<(), String> {
    let mut presets = load()?;
    presets.retain(|existing| existing.id != preset_id);
    write(&presets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_from_older_versions_fill_missing_fields() {
        let preset: VoicePreset = serde_json::from_str(
            r#"{"id": "preset-1", "name": "Old", "model_key": "piper-en_US-ryan-low", "pitch": -2.0}"#,
        )
        .unwrap();
        assert_eq!(preset.tuning.noise_scale, 0.667);
        assert_eq!(preset.tuning.opening_chime, "none");
        assert_eq!(preset.tuning.pitch, -2.0);
    }
}
