use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: u64,
    pub text: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct History {
    pub items: Vec<HistoryItem>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            items: Vec::new(),
        }
    }
}

fn get_history_file_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = dirs::config_dir()
        .ok_or("Could not find config directory")?;
    path.push("voquill");
    
    // Create directory if it doesn't exist
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    
    path.push("history.json");
    Ok(path)
}

pub fn load_history() -> Result<History, Box<dyn std::error::Error>> {
    let path = get_history_file_path()?;
    
    if !path.exists() {
        return Ok(History::default());
    }
    
    let content = fs::read_to_string(path)?;
    let history: History = serde_json::from_str(&content)?;
    Ok(history)
}

pub fn save_history(history: &History) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_history_file_path()?;
    let content = serde_json::to_string_pretty(history)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn add_history_item(text: &str) -> Result<HistoryItem, Box<dyn std::error::Error>> {
    let mut history = load_history()?;
    
    // Store as ISO 8601 UTC timestamp for easy parsing in frontend
    let timestamp = Utc::now().to_rfc3339();
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;
    
    let item = HistoryItem {
        id,
        text: text.to_string(),
        timestamp,
    };
    
    // Add to beginning of list (most recent first)
    history.items.insert(0, item.clone());
    
    // Keep only last 100 items
    if history.items.len() > 100 {
        history.items.truncate(100);
    }
    
    save_history(&history)?;
    Ok(item)
}

pub fn clear_history() -> Result<(), Box<dyn std::error::Error>> {
    let history = History::default();
    save_history(&history)?;
    Ok(())
}
