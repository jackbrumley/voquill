//! On-disk persistence for `config.json`: loading (with legacy-field
//! migration), atomic saving, and recovery from a file that cannot be parsed.
//!
//! Loading never silently discards the user's settings. A file that exists
//! but cannot be read is a hard error, so the caller must not start and
//! overwrite it. A file that cannot be parsed is moved aside to a timestamped
//! backup before defaults are written in its place, and the backup location is
//! reported back so it can be surfaced to the user.

use super::Config;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

/// Serializes writers so concurrent saves never share the temp file.
static SAVE_LOCK: Mutex<()> = Mutex::new(());

pub struct LoadedConfig {
    pub config: Config,
    /// Set when the file on disk could not be parsed and was moved aside.
    pub recovery: Option<ConfigRecovery>,
}

pub struct ConfigRecovery {
    pub backup_path: PathBuf,
    pub parse_error: String,
}

pub fn load_config() -> Result<LoadedConfig, String> {
    let config_path = crate::paths::config_file()?;

    let config_bytes = match fs::read(&config_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            log_info!("No config file at {:?}; creating defaults", config_path);
            let default_config = Config::default();
            save_config(&default_config)?;
            return Ok(LoadedConfig {
                config: default_config,
                recovery: None,
            });
        }
        Err(error) => {
            return Err(format!(
                "Could not read settings file {}: {}",
                config_path.display(),
                error
            ));
        }
    };

    match parse_config(&config_bytes) {
        Ok(config) => {
            // Persist normalization and legacy migrations to keep the file clean.
            save_config(&config)?;
            Ok(LoadedConfig {
                config,
                recovery: None,
            })
        }
        Err(parse_error) => {
            let backup_path = move_aside_invalid_config(&config_path, &parse_error)?;
            let default_config = Config::default();
            save_config(&default_config)?;
            Ok(LoadedConfig {
                config: default_config,
                recovery: Some(ConfigRecovery {
                    backup_path,
                    parse_error,
                }),
            })
        }
    }
}

pub fn save_config(config: &Config) -> Result<(), String> {
    let config_path = crate::paths::config_file()?;
    log_info!("Attempting to save config to: {:?}", config_path);

    let mut normalized_config = config.clone();
    normalized_config.normalize();
    let config_text = serde_json::to_string_pretty(&normalized_config)
        .map_err(|error| format!("Failed to serialize config: {}", error))?;
    log_info!(
        "Config summary: mode={:?}, engine={}, model={}, hotkey={}, audio_device={:?}, recording_logs={}, input_sensitivity={:.2}, diarization_cluster_threshold={:.2}",
        normalized_config.transcription_mode,
        normalized_config.local_engine,
        normalized_config.local_model_size,
        normalized_config.hotkey,
        normalized_config.audio_device,
        normalized_config.enable_recording_logs,
        normalized_config.input_sensitivity,
        normalized_config.diarization_cluster_threshold
    );

    write_atomically(&config_path, config_text.as_bytes()).map_err(|error| {
        format!(
            "Failed to write settings file {}: {}",
            config_path.display(),
            error
        )
    })?;
    log_info!("Config saved successfully to: {:?}", config_path);
    Ok(())
}

fn parse_config(config_bytes: &[u8]) -> Result<Config, String> {
    let config_value: serde_json::Value =
        serde_json::from_slice(config_bytes).map_err(|error| error.to_string())?;
    // Serde would otherwise accept e.g. `[]` as an all-defaults Config,
    // silently resetting every setting.
    let serde_json::Value::Object(mut fields) = config_value else {
        return Err("settings file is not a JSON object".to_string());
    };
    migrate_legacy_fields(&mut fields);
    let mut config = serde_json::from_value::<Config>(serde_json::Value::Object(fields))
        .map_err(|error| error.to_string())?;
    config.normalize();
    Ok(config)
}

fn migrate_legacy_fields(fields: &mut serde_json::Map<String, serde_json::Value>) {
    // Migrate legacy linux_portal_hotkey into hotkey, then drop the legacy field
    if let Some(serde_json::Value::String(portal_hotkey)) = fields.remove("linux_portal_hotkey") {
        if !portal_hotkey.trim().is_empty() {
            fields.insert(
                "hotkey".to_string(),
                serde_json::Value::String(portal_hotkey),
            );
        }
    }

    let normalized_hotkey = fields
        .get("hotkey")
        .and_then(|value| value.as_str())
        .and_then(normalize_legacy_portal_hotkey);
    if let Some(normalized_hotkey) = normalized_hotkey {
        fields.insert(
            "hotkey".to_string(),
            serde_json::Value::String(normalized_hotkey),
        );
    }
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

/// Moves an unparseable config file to `config.json.invalid-<timestamp>` so
/// the user's settings survive for manual repair instead of being overwritten.
fn move_aside_invalid_config(config_path: &Path, parse_error: &str) -> Result<PathBuf, String> {
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let backup_path = config_path.with_file_name(format!("config.json.invalid-{}", timestamp));
    fs::rename(config_path, &backup_path).map_err(|error| {
        format!(
            "Settings file {} could not be parsed ({}), and moving it aside to {} failed: {}",
            config_path.display(),
            parse_error,
            backup_path.display(),
            error
        )
    })?;
    log_warn!(
        "Settings file {:?} could not be parsed ({}); moved it to {:?} and starting with defaults",
        config_path,
        parse_error,
        backup_path
    );
    Ok(backup_path)
}

/// Writes to a sibling temp file, flushes it to disk, then renames it over the
/// target, so a crash or power loss mid-save leaves either the old file or the
/// new one, never a truncated one.
fn write_atomically(target_path: &Path, contents: &[u8]) -> io::Result<()> {
    let _save_guard = SAVE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let temp_path = target_path.with_extension("json.tmp");
    let mut temp_file = fs::File::create(&temp_path)?;
    temp_file.write_all(contents)?;
    temp_file.sync_all()?;
    drop(temp_file);
    fs::rename(&temp_path, target_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_rejects_truncated_json() {
        assert!(parse_config(br#"{"hotkey": "ctrl+space", "dicti"#).is_err());
    }

    #[test]
    fn parse_config_rejects_non_object_json() {
        assert!(parse_config(b"[]").is_err());
        assert!(parse_config(b"null").is_err());
    }

    #[test]
    fn parse_config_migrates_legacy_portal_hotkey() {
        let config =
            parse_config(br#"{"linux_portal_hotkey": "Press <Control><Shift>space"}"#).unwrap();
        assert_eq!(config.hotkey, "ctrl+shift+space");
    }

    #[test]
    fn write_atomically_replaces_existing_file_and_leaves_no_temp() {
        let directory = std::env::temp_dir().join(format!(
            "voquill-config-storage-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&directory).unwrap();
        let target_path = directory.join("config.json");
        fs::write(&target_path, b"old").unwrap();

        write_atomically(&target_path, b"new").unwrap();

        assert_eq!(fs::read(&target_path).unwrap(), b"new");
        assert!(!directory.join("config.json.tmp").exists());
        fs::remove_dir_all(&directory).unwrap();
    }
}
