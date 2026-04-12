use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static SESSION_LOG_FILE: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
static SESSION_LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn get_session_log_path() -> Result<PathBuf, String> {
    let debug_dir = dirs::config_dir()
        .ok_or_else(|| "Could not find config directory".to_string())?
        .join("foss-voquill")
        .join("debug");
    fs::create_dir_all(&debug_dir).map_err(|error| error.to_string())?;
    Ok(debug_dir.join("session.log"))
}

pub fn get_app_config_root_dir() -> Result<PathBuf, String> {
    let root_dir = dirs::config_dir()
        .ok_or_else(|| "Could not find config directory".to_string())?
        .join("foss-voquill");

    fs::create_dir_all(&root_dir).map_err(|error| error.to_string())?;
    Ok(root_dir)
}

pub fn clear_directory_contents(
    path: &std::path::Path,
    preserve_filenames: &[&str],
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let entry_path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if preserve_filenames.iter().any(|name| *name == file_name) {
            continue;
        }

        if entry_path.is_dir() {
            fs::remove_dir_all(&entry_path).map_err(|error| error.to_string())?;
        } else {
            fs::remove_file(&entry_path).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

pub fn truncate_session_log_with_header() -> Result<(), String> {
    if let Some(lock) = SESSION_LOG_FILE.get() {
        if let Ok(mut maybe_file) = lock.lock() {
            if let Some(file) = maybe_file.as_mut() {
                file.set_len(0).map_err(|error| error.to_string())?;
                file.seek(SeekFrom::Start(0))
                    .map_err(|error| error.to_string())?;
                writeln!(
                    file,
                    "[{}] SESSION RESET | version={}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    env!("CARGO_PKG_VERSION")
                )
                .map_err(|error| error.to_string())?;
                return Ok(());
            }
        }
    }

    Err("Session log file handle unavailable".to_string())
}

pub fn initialize_session_logging() {
    let log_path = match get_session_log_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to initialize session log path: {}", error);
            return;
        }
    };

    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
    {
        Ok(mut file) => {
            let startup_header = format!(
                "[{}] SESSION START | version={}\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                env!("CARGO_PKG_VERSION")
            );
            let _ = file.write_all(startup_header.as_bytes());
            let _ = SESSION_LOG_PATH.set(log_path);
            let _ = SESSION_LOG_FILE.set(Mutex::new(Some(file)));
        }
        Err(error) => {
            eprintln!("Failed to open session log file: {}", error);
        }
    }
}

pub fn append_session_log(level: &str, timestamp: &str, message: &str) {
    if let Some(lock) = SESSION_LOG_FILE.get() {
        if let Ok(mut maybe_file) = lock.lock() {
            if let Some(file) = maybe_file.as_mut() {
                let _ = writeln!(file, "[{}] {}: {}", timestamp, level, message);
            }
        }
    }
}

pub fn resolve_session_log_path() -> Result<PathBuf, String> {
    SESSION_LOG_PATH
        .get()
        .cloned()
        .or_else(|| get_session_log_path().ok())
        .ok_or_else(|| "Session log path unavailable".to_string())
}
