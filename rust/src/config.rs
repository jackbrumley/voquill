use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub openai_api_key: String,
    pub hotkey: String,
    pub typing_speed_interval: f64,
    pub pixels_from_bottom: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openai_api_key: "your_api_key_here".to_string(),
            hotkey: "ctrl+space".to_string(),
            typing_speed_interval: 0.01,
            pixels_from_bottom: 100,
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
        let config_str = fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_str)?;
        Ok(config)
    } else {
        // Create default config file
        let default_config = Config::default();
        save_config(&default_config)?;
        Ok(default_config)
    }
}

pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = get_config_path()?;
    println!("Attempting to save config to: {:?}", config_path);
    
    let config_str = serde_json::to_string_pretty(config)?;
    println!("Config JSON: {}", config_str);
    
    fs::write(&config_path, config_str)?;
    println!("Config saved successfully to: {:?}", config_path);
    Ok(())
}
