use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub openai_api_key: String,
    pub api_url: String,
    pub hotkey: String,
    pub typing_speed_interval: f64,
    pub pixels_from_bottom: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openai_api_key: "your_api_key_here".to_string(),
            api_url: "https://api.openai.com/v1/audio/transcriptions".to_string(),
            hotkey: "ctrl+space".to_string(),
            typing_speed_interval: 0.01,
            pixels_from_bottom: 50,
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
        
        // Try to parse as current Config struct
        match serde_json::from_str::<Config>(&config_str) {
            Ok(config) => Ok(config),
            Err(_) => {
                // Config might be missing new fields, try to migrate
                println!("Config migration needed - updating to latest format");
                
                // Parse as a generic JSON value to handle missing fields
                let mut config_value: serde_json::Value = serde_json::from_str(&config_str)?;
                
                // Add missing api_url field if it doesn't exist
                if !config_value.get("api_url").is_some() {
                    config_value["api_url"] = serde_json::Value::String(
                        "https://api.openai.com/v1/audio/transcriptions".to_string()
                    );
                }
                
                // Parse the migrated config
                let migrated_config: Config = serde_json::from_value(config_value)?;
                
                // Save the migrated config
                save_config(&migrated_config)?;
                println!("Config migrated successfully");
                
                Ok(migrated_config)
            }
        }
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
    println!("Attempting to save config to: {:?}", config_path);
    
    let config_str = serde_json::to_string_pretty(config)?;
    println!("Config JSON: {}", config_str);
    
    fs::write(&config_path, config_str)?;
    println!("Config saved successfully to: {:?}", config_path);
    Ok(())
}
