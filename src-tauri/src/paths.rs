//! Single source of truth for all on-disk storage locations.
//!
//! Everything Voquill persists — config, history, models, sidecar binaries,
//! the python-runner, and debug logs — lives under one root:
//! `~/.config/voquill-app` on every platform (Linux honors `XDG_CONFIG_HOME`).

use std::fs;
use std::path::{Path, PathBuf};

const APP_DIR_NAME: &str = "voquill-app";
const LEGACY_DIR_NAME: &str = "foss-voquill";

/// Root directory for all Voquill data.
///
/// Linux: `$XDG_CONFIG_HOME/voquill-app` (or `~/.config/voquill-app`).
/// Windows/macOS: `~/.config/voquill-app` — deliberately kept out of
/// `%APPDATA%` so multi-GB models never roam with enterprise profiles.
pub fn app_root() -> Result<PathBuf, String> {
    #[cfg(target_os = "linux")]
    let base = dirs::config_dir().ok_or("Could not find config directory")?;
    #[cfg(not(target_os = "linux"))]
    let base = dirs::home_dir()
        .ok_or("Could not find home directory")?
        .join(".config");
    Ok(base.join(APP_DIR_NAME))
}

/// App root, created on disk if missing.
pub fn ensure_app_root() -> Result<PathBuf, String> {
    let root = app_root()?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    Ok(root)
}

pub fn config_file() -> Result<PathBuf, String> {
    Ok(ensure_app_root()?.join("config.json"))
}

pub fn history_db() -> Result<PathBuf, String> {
    Ok(ensure_app_root()?.join("history.db"))
}

pub fn models_dir() -> Result<PathBuf, String> {
    let dir = app_root()?.join("models");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

#[allow(dead_code)]
pub fn tts_models_dir() -> Result<PathBuf, String> {
    let dir = models_dir()?.join("tts");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

#[allow(dead_code)]
pub fn sounds_dir() -> Result<PathBuf, String> {
    let dir = app_root()?.join("sounds");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

pub fn macro_sounds_dir() -> Result<PathBuf, String> {
    let dir = sounds_dir()?.join("macros");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

pub fn python_runner_dir() -> Result<PathBuf, String> {
    Ok(app_root()?.join("python-runner"))
}

pub fn voice_presets_file() -> Result<PathBuf, String> {
    Ok(ensure_app_root()?.join("voice_presets.json"))
}

pub fn debug_dir() -> Result<PathBuf, String> {
    let dir = app_root()?.join("debug");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

pub fn debug_recordings_dir() -> Result<PathBuf, String> {
    let dir = debug_dir()?.join("recordings");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

/// Temporary directory for runtime artifacts (diarization temp WAV files, etc.).
/// Cleaned up by the OS across reboots.
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir().join(APP_DIR_NAME)
}

/// One-time migration from the legacy location (`<os config dir>/foss-voquill`,
/// e.g. `%APPDATA%\foss-voquill` on Windows) to the unified
/// `~/.config/voquill-app` root.
///
/// Returns a human-readable report when something noteworthy happened
/// (migrated / kept-new / failed), or `None` when there was nothing to
/// migrate. Every outcome is explicit: the legacy directory is only removed
/// after its contents are fully in place at the new root.
pub fn migrate_legacy_location() -> Option<String> {
    let legacy_root = dirs::config_dir().map(|dir| dir.join(LEGACY_DIR_NAME))?;
    if !legacy_root.exists() {
        return None;
    }

    let new_root = match app_root() {
        Ok(root) => root,
        Err(error) => {
            return Some(format!(
                "legacy data found at {} but new root unavailable: {}",
                legacy_root.display(),
                error
            ));
        }
    };

    if new_root.exists() {
        return Some(format!(
            "both legacy ({}) and new ({}) data directories exist; keeping new, legacy left untouched",
            legacy_root.display(),
            new_root.display()
        ));
    }

    // fs::rename requires the destination parent to exist (fresh Windows
    // machines have no `~/.config` yet).
    if let Some(parent) = new_root.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            return Some(format!(
                "failed to create {} for migration: {}",
                parent.display(),
                error
            ));
        }
    }

    match fs::rename(&legacy_root, &new_root) {
        Ok(()) => Some(format!(
            "migrated data from {} to {}",
            legacy_root.display(),
            new_root.display()
        )),
        Err(rename_error) => match copy_tree(&legacy_root, &new_root) {
            Ok(()) => {
                if let Err(remove_error) = fs::remove_dir_all(&legacy_root) {
                    return Some(format!(
                        "copied data to {} but failed to remove legacy {}: {}",
                        new_root.display(),
                        legacy_root.display(),
                        remove_error
                    ));
                }
                Some(format!(
                    "migrated data from {} to {} (copy fallback after rename failed: {})",
                    legacy_root.display(),
                    new_root.display(),
                    rename_error
                ))
            }
            Err(copy_error) => Some(format!(
                "FAILED to migrate data from {} to {} (rename: {}; copy: {}); starting fresh, legacy left untouched",
                legacy_root.display(),
                new_root.display(),
                rename_error,
                copy_error
            )),
        },
    }
}

/// Cleans up legacy or obsolete autostart desktop entries and registry keys
/// from previous versions of the application (e.g. `org.voquill.app.desktop`,
/// `org.voquill.foss.desktop`, `Voquill.desktop`).
pub fn cleanup_legacy_autostart_entries() -> Vec<String> {
    let mut cleaned = Vec::new();

    #[cfg(target_os = "linux")]
    {
        if let Some(config_dir) = dirs::config_dir() {
            let autostart_dir = config_dir.join("autostart");
            if autostart_dir.is_dir() {
                let legacy_files = [
                    "org.voquill.app.desktop",
                    "org.voquill.foss.desktop",
                    "Voquill.desktop",
                    "org.voquill.desktop.desktop",
                ];
                for file_name in legacy_files {
                    let path = autostart_dir.join(file_name);
                    if path.exists() && fs::remove_file(&path).is_ok() {
                        cleaned.push(format!("removed legacy autostart file: {}", path.display()));
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(run_key) = hkcu.open_subkey_with_flags(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Run",
            KEY_SET_VALUE,
        ) {
            let legacy_names = [
                "Voquill",
                "org.voquill.app",
                "org.voquill.foss",
                "foss-voquill",
            ];
            for name in legacy_names {
                if run_key.delete_value(name).is_ok() {
                    cleaned.push(format!(
                        "removed legacy Windows autostart registry key: {}",
                        name
                    ));
                }
            }
        }
    }

    cleaned
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}
