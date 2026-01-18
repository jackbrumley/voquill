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
}

fn default_api_key() -> String { "your_api_key_here".to_string() }
fn default_api_url() -> String { "https://api.openai.com/v1/audio/transcriptions".to_string() }
fn default_api_model() -> String { "whisper-1".to_string() }
fn default_transcription_mode() -> TranscriptionMode { TranscriptionMode::Local }
fn default_local_model_size() -> String { "base".to_string() }
fn default_hotkey() -> String { "ctrl+space".to_string() }
fn default_typing_speed() -> f64 { 0.001 }
fn default_key_press_duration() -> u64 { 2 }
fn default_pixels_from_bottom() -> i32 { 100 }
fn default_audio_device() -> Option<String> { Some("default".to_string()) }
fn default_debug_mode() -> bool { false }
fn default_enable_recording_logs() -> bool { false }
fn default_input_sensitivity() -> f32 { 1.0 }
fn default_output_method() -> OutputMethod { OutputMethod::Typewriter }
fn default_copy_on_typewriter() -> bool { false }
fn default_language() -> String { "auto".to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            openai_api_key: default_api_key(),
            api_url: default_api_url(),
            api_model: default_api_model(),
            transcription_mode: default_transcription_mode(),
            local_model_size: default_local_model_size(),
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
        }
    }
}

pub fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("voquill");
    
    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("config.json"))
}

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;
    
    if config_path.exists() {
        let config_str = fs::read_to_string(&config_path)?;
        
        // Try to parse as current Config struct - serde(default) handles missing fields
        let config = serde_json::from_str::<Config>(&config_str)?;
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
    log_info!("Config JSON: {}", config_str);
    
    fs::write(&config_path, config_str)?;
    log_info!("Config saved successfully to: {:?}", config_path);
    Ok(())
}
