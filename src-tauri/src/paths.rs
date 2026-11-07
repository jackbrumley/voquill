//! Single source of truth for all on-disk storage locations.
//!
//! Everything Voquill persists — config, history, models, sidecar binaries,
//! the python-runner, and debug logs — lives under one root:
//! `~/.config/voquill-app` on every platform (Linux honors `XDG_CONFIG_HOME`).

use std::fs;
use std::path::{Path, PathBuf};

const APP_DIR_NAME: &str = "voquill-app";
const LEGACY_DIR_NAME: &str = "foss-voquill";
const SETTINGS_FILE_NAME: &str = "settings.json";
/// Name older versions (up to 1.9.5) gave the settings file (see
/// `migrate_legacy_settings_file`).
const LEGACY_SETTINGS_FILE_NAME: &str = "config.json";

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

pub fn settings_file() -> Result<PathBuf, String> {
    Ok(ensure_app_root()?.join(SETTINGS_FILE_NAME))
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
    let new_root = match app_root() {
        Ok(root) => root,
        Err(error) if legacy_root.exists() => {
            return Some(format!(
                "legacy data found at {} but new root unavailable: {}",
                legacy_root.display(),
                error
            ));
        }
        Err(_) => return None,
    };
    migrate_legacy_root(&legacy_root, &new_root)
}

/// The migration itself, between explicit roots (testable).
fn migrate_legacy_root(legacy_root: &Path, new_root: &Path) -> Option<String> {
    if !legacy_root.exists() {
        return None;
    }

    if new_root.exists() {
        if holds_only_installer_logs(new_root) {
            return Some(merge_into_installer_only_root(legacy_root, new_root));
        }
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

    match fs::rename(legacy_root, new_root) {
        Ok(()) => Some(format!(
            "migrated data from {} to {}",
            legacy_root.display(),
            new_root.display()
        )),
        Err(rename_error) => match copy_tree(legacy_root, new_root) {
            Ok(()) => {
                if let Err(remove_error) = fs::remove_dir_all(legacy_root) {
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

/// True when the new root holds nothing but `debug/`. The installers write
/// `debug/update.log` there before the new app first starts, so an upgrade
/// from a legacy-location version finds this root already present without
/// any of the user's data in it.
fn holds_only_installer_logs(new_root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(new_root) else {
        return false;
    };
    entries
        .map(|entry| entry.map(|entry| entry.file_name() == "debug"))
        .all(|is_debug| matches!(is_debug, Ok(true)))
}

/// Merges the legacy root into a new root that holds only installer logs,
/// then removes the legacy root. Files already in the new root (the
/// installer's `update.log`) win over legacy files of the same name.
fn merge_into_installer_only_root(legacy_root: &Path, new_root: &Path) -> String {
    match merge_tree(legacy_root, new_root) {
        Ok(()) => match fs::remove_dir_all(legacy_root) {
            Ok(()) => format!(
                "merged data from {} into {} (which held only installer logs)",
                legacy_root.display(),
                new_root.display()
            ),
            Err(remove_error) => format!(
                "merged data into {} but failed to remove legacy {}: {}",
                new_root.display(),
                legacy_root.display(),
                remove_error
            ),
        },
        Err(merge_error) => format!(
            "FAILED to merge data from {} into {} (which held only installer logs): {}; legacy left in place",
            legacy_root.display(),
            new_root.display(),
            merge_error
        ),
    }
}

/// Moves every entry of `source` into `destination`, recursing into
/// directories present in both and keeping the destination's file when both
/// have one. Copies when a rename fails (e.g. across volumes).
fn merge_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if destination_path.exists() {
            if source_path.is_dir() && destination_path.is_dir() {
                merge_tree(&source_path, &destination_path)?;
            }
            continue;
        }
        if fs::rename(&source_path, &destination_path).is_err() {
            if source_path.is_dir() {
                copy_tree(&source_path, &destination_path)?;
            } else {
                fs::copy(&source_path, &destination_path)?;
            }
        }
    }
    Ok(())
}

/// What `migrate_legacy_settings_file` did, so the caller logs each outcome
/// at the right level.
pub enum SettingsFileMigration {
    Renamed(String),
    Warning(String),
}

/// One-time rename of the legacy settings file (`config.json`) to
/// `settings.json`, the name every app of the App Standard uses. Runs after
/// `migrate_legacy_location` and before the settings load. Renames only when
/// the old file exists and the new one doesn't; when both exist, the new one
/// wins and the old one is left untouched. Returns `None` when there was
/// nothing to migrate.
pub fn migrate_legacy_settings_file() -> Option<SettingsFileMigration> {
    migrate_settings_file_in(&app_root().ok()?)
}

fn migrate_settings_file_in(root: &Path) -> Option<SettingsFileMigration> {
    let legacy_path = root.join(LEGACY_SETTINGS_FILE_NAME);
    if !legacy_path.exists() {
        return None;
    }
    let settings_path = root.join(SETTINGS_FILE_NAME);

    if settings_path.exists() {
        return Some(SettingsFileMigration::Warning(format!(
            "both legacy {} and {} exist; keeping {}, legacy left untouched",
            legacy_path.display(),
            settings_path.display(),
            settings_path.display()
        )));
    }

    match fs::rename(&legacy_path, &settings_path) {
        Ok(()) => Some(SettingsFileMigration::Renamed(format!(
            "renamed {} to {}",
            legacy_path.display(),
            settings_path.display()
        ))),
        Err(error) => Some(SettingsFileMigration::Warning(format!(
            "FAILED to rename {} to {}: {}; starting with defaults, legacy left untouched",
            legacy_path.display(),
            settings_path.display(),
            error
        ))),
    }
}

/// Cleans up legacy or obsolete autostart desktop entries and registry keys
/// from previous versions of the application (e.g. `org.voquill.app.desktop`,
/// `org.voquill.foss.desktop`, `Voquill.desktop`).
#[cfg(target_os = "linux")]
pub fn cleanup_legacy_autostart_entries() -> Vec<String> {
    let mut cleaned = Vec::new();
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
    cleaned
}

/// Cleans up legacy Windows autostart registry values written by previous
/// versions of the application.
#[cfg(target_os = "windows")]
pub fn cleanup_legacy_autostart_entries() -> Vec<String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    let mut cleaned = Vec::new();
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
    cleaned
}

/// No earlier release shipped autostart entries on other platforms.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn cleanup_legacy_autostart_entries() -> Vec<String> {
    Vec::new()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_dir() -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("voquill-paths-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn legacy_root_merges_into_root_holding_only_installer_logs() {
        let scratch = scratch_dir();
        let legacy_root = scratch.join("foss-voquill");
        let new_root = scratch.join("voquill-app");
        fs::create_dir_all(legacy_root.join("debug")).unwrap();
        fs::create_dir_all(legacy_root.join("models")).unwrap();
        fs::write(legacy_root.join("config.json"), b"legacy settings").unwrap();
        fs::write(legacy_root.join("models/model.bin"), b"model").unwrap();
        fs::write(legacy_root.join("debug/update.log"), b"old log").unwrap();
        fs::write(legacy_root.join("debug/session.log"), b"old session").unwrap();
        fs::create_dir_all(new_root.join("debug")).unwrap();
        fs::write(new_root.join("debug/update.log"), b"installer log").unwrap();

        let report = migrate_legacy_root(&legacy_root, &new_root).unwrap();

        assert!(report.starts_with("merged"), "{report}");
        assert!(!legacy_root.exists());
        assert_eq!(
            fs::read(new_root.join("config.json")).unwrap(),
            b"legacy settings"
        );
        assert_eq!(
            fs::read(new_root.join("models/model.bin")).unwrap(),
            b"model"
        );
        assert_eq!(
            fs::read(new_root.join("debug/update.log")).unwrap(),
            b"installer log"
        );
        assert_eq!(
            fs::read(new_root.join("debug/session.log")).unwrap(),
            b"old session"
        );
        fs::remove_dir_all(&scratch).unwrap();
    }

    #[test]
    fn populated_new_root_keeps_legacy_root_untouched() {
        let scratch = scratch_dir();
        let legacy_root = scratch.join("foss-voquill");
        let new_root = scratch.join("voquill-app");
        fs::create_dir_all(&legacy_root).unwrap();
        fs::write(legacy_root.join("config.json"), b"legacy settings").unwrap();
        fs::create_dir_all(new_root.join("debug")).unwrap();
        fs::write(new_root.join("settings.json"), b"new settings").unwrap();

        let report = migrate_legacy_root(&legacy_root, &new_root).unwrap();

        assert!(report.starts_with("both legacy"), "{report}");
        assert!(legacy_root.join("config.json").exists());
        assert!(!new_root.join("config.json").exists());
        fs::remove_dir_all(&scratch).unwrap();
    }

    #[test]
    fn legacy_settings_file_is_renamed_with_its_contents() {
        let root = scratch_dir();
        fs::write(root.join("config.json"), b"legacy settings").unwrap();

        let outcome = migrate_settings_file_in(&root);

        assert!(matches!(outcome, Some(SettingsFileMigration::Renamed(_))));
        assert!(!root.join("config.json").exists());
        assert_eq!(
            fs::read(root.join("settings.json")).unwrap(),
            b"legacy settings"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn existing_settings_file_wins_and_legacy_file_is_left_untouched() {
        let root = scratch_dir();
        fs::write(root.join("config.json"), b"legacy settings").unwrap();
        fs::write(root.join("settings.json"), b"new settings").unwrap();

        let outcome = migrate_settings_file_in(&root);

        assert!(matches!(outcome, Some(SettingsFileMigration::Warning(_))));
        assert_eq!(
            fs::read(root.join("settings.json")).unwrap(),
            b"new settings"
        );
        assert_eq!(
            fs::read(root.join("config.json")).unwrap(),
            b"legacy settings"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn nothing_to_migrate_without_a_legacy_settings_file() {
        let root = scratch_dir();
        fs::write(root.join("settings.json"), b"new settings").unwrap();

        assert!(migrate_settings_file_in(&root).is_none());
        assert_eq!(
            fs::read(root.join("settings.json")).unwrap(),
            b"new settings"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn legacy_root_moves_when_new_root_is_absent() {
        let scratch = scratch_dir();
        let legacy_root = scratch.join("foss-voquill");
        let new_root = scratch.join("voquill-app");
        fs::create_dir_all(&legacy_root).unwrap();
        fs::write(legacy_root.join("config.json"), b"legacy settings").unwrap();

        let report = migrate_legacy_root(&legacy_root, &new_root).unwrap();

        assert!(report.starts_with("migrated"), "{report}");
        assert!(!legacy_root.exists());
        assert!(new_root.join("config.json").exists());
        fs::remove_dir_all(&scratch).unwrap();
    }
}
