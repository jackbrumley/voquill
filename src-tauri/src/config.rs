use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutputMethod {
    Typewriter,
    Clipboard,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranscriptionMode {
    API,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_api_key")]
    pub openai_api_key: String,
    #[serde(default = "default_api_url")]
    pub api_url: String,
    #[serde(default = "default_api_model")]
    pub api_model: String,
    #[serde(default = "default_transcription_mode")]
    pub transcription_mode: TranscriptionMode,
    #[serde(default = "default_local_model_size")]
    pub local_model_size: String,
    #[serde(default = "default_local_engine")]
    pub local_engine: String,
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default = "default_typing_speed")]
    pub typing_speed_interval: f64,
    #[serde(default = "default_key_press_duration")]
    pub key_press_duration_ms: u64,
    #[serde(default = "default_pixels_from_bottom")]
    pub pixels_from_bottom: i32,
    #[serde(default = "default_audio_device")]
    pub audio_device: Option<String>,
    #[serde(default = "default_debug_mode")]
    pub debug_mode: bool,
    #[serde(default = "default_enable_recording_logs")]
    pub enable_recording_logs: bool,
    #[serde(default = "default_input_sensitivity")]
    pub input_sensitivity: f32,
    #[serde(default = "default_output_method")]
    pub output_method: OutputMethod,
    #[serde(default = "default_copy_on_typewriter")]
    pub copy_on_typewriter: bool,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub shortcuts_token: Option<String>,
    #[serde(default)]
    pub input_token: Option<String>,
    #[serde(default = "default_enable_gpu")]
    pub enable_gpu: bool,
}

fn default_api_key() -> String {
    "your_api_key_here".to_string()
}
fn default_api_url() -> String {
    "https://api.openai.com/v1/audio/transcriptions".to_string()
}
fn default_api_model() -> String {
    "whisper-1".to_string()
}
fn default_transcription_mode() -> TranscriptionMode {
    TranscriptionMode::Local
}
fn default_local_model_size() -> String {
    "base".to_string()
}
fn default_local_engine() -> String {
    "Whisper.cpp".to_string()
}
fn default_hotkey() -> String {
    "ctrl+shift+space".to_string()
}
fn default_typing_speed() -> f64 {
    0.001
}
fn default_key_press_duration() -> u64 {
    2
}
fn default_pixels_from_bottom() -> i32 {
    150
}
fn default_audio_device() -> Option<String> {
    Some("default".to_string())
}
fn default_debug_mode() -> bool {
    false
}
fn default_enable_recording_logs() -> bool {
    false
}
fn default_input_sensitivity() -> f32 {
    1.0
}
fn default_output_method() -> OutputMethod {
    OutputMethod::Typewriter
}
fn default_copy_on_typewriter() -> bool {
    false
}
fn default_language() -> String {
    "auto".to_string()
}
fn default_enable_gpu() -> bool {
    false
}

fn normalize_legacy_portal_hotkey(hotkey: &str) -> Option<String> {
    let trimmed = hotkey.trim();
    let lower = trimmed.to_lowercase();

    if !lower.starts_with("press <") {
        return None;
    }

    let mut modifiers: Vec<&str> = Vec::new();
    if lower.contains("<control>") {
        modifiers.push("ctrl");
    }
    if lower.contains("<shift>") {
        modifiers.push("shift");
    }
    if lower.contains("<alt>") {
        modifiers.push("alt");
    }
    if lower.contains("<super>") || lower.contains("<logo>") {
        modifiers.push("super");
    }

    let key_start_index = lower.rfind('>').map(|index| index + 1).unwrap_or(0);
    let key = lower[key_start_index..].trim();

    if key.is_empty() {
        return None;
    }

    let mut normalized = modifiers
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>();
    normalized.push(key.to_string());

    Some(normalized.join("+"))
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openai_api_key: default_api_key(),
            api_url: default_api_url(),
            api_model: default_api_model(),
            transcription_mode: default_transcription_mode(),
            local_model_size: default_local_model_size(),
            local_engine: default_local_engine(),
            hotkey: default_hotkey(),
            typing_speed_interval: default_typing_speed(),
            key_press_duration_ms: default_key_press_duration(),
            pixels_from_bottom: default_pixels_from_bottom(),
            audio_device: default_audio_device(),
            debug_mode: default_debug_mode(),
            enable_recording_logs: default_enable_recording_logs(),
            input_sensitivity: default_input_sensitivity(),
            output_method: default_output_method(),
            copy_on_typewriter: default_copy_on_typewriter(),
            language: default_language(),
            shortcuts_token: None,
            input_token: None,
            enable_gpu: default_enable_gpu(),
        }
    }
}

pub fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("foss-voquill");

    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("config.json"))
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;

    if config_path.exists() {
        let config_str = fs::read_to_string(&config_path)?;

        // Migrate legacy linux_portal_hotkey into hotkey, then drop the legacy field
        let mut config_value: serde_json::Value = serde_json::from_str(&config_str)?;
        if let Some(portal_hotkey) = config_value
            .get("linux_portal_hotkey")
            .and_then(|value| value.as_str())
        {
            if !portal_hotkey.trim().is_empty() {
                config_value["hotkey"] = serde_json::Value::String(portal_hotkey.to_string());
            }
        }
        if let Some(obj) = config_value.as_object_mut() {
            obj.remove("linux_portal_hotkey");

            if let Some(hotkey) = obj.get("hotkey").and_then(|value| value.as_str()) {
                if let Some(normalized_hotkey) = normalize_legacy_portal_hotkey(hotkey) {
                    obj.insert(
                        "hotkey".to_string(),
                        serde_json::Value::String(normalized_hotkey),
                    );
                }
            }
        }

        let config = serde_json::from_value::<Config>(config_value)?;
        // Persist migration to disk to keep config clean
        save_config(&config)?;
        Ok(config)
    } else {
        // Create default config file
        let default_config = Config::default();
        save_config(&default_config)?;
        Ok(default_config)
    }
}

pub fn is_first_launch() -> Result<bool, Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;

    // If config file doesn't exist, it's definitely first launch
    if !config_path.exists() {
        return Ok(true);
    }

    // If config exists but API key is still default, treat as first launch
    let config = load_config()?;
    Ok(config.openai_api_key == "your_api_key_here" || config.openai_api_key.is_empty())
}

pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;
    log_info!("Attempting to save config to: {:?}", config_path);

    let config_str = serde_json::to_string_pretty(config)?;
    log_info!(
        "Config summary: mode={:?}, engine={}, model={}, hotkey={}, audio_device={:?}, debug_mode={}, recording_logs={}, gpu={}",
        config.transcription_mode,
        config.local_engine,
        config.local_model_size,
        config.hotkey,
        config.audio_device,
        config.debug_mode,
        config.enable_recording_logs,
        config.enable_gpu
    );

    fs::write(&config_path, config_str)?;
    log_info!("Config saved successfully to: {:?}", config_path);
    Ok(())
}
