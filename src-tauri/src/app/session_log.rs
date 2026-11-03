use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static SESSION_LOG_FILE: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
static SESSION_LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

static PERSISTENCE_ENABLED: AtomicBool = AtomicBool::new(true);

/// The current local time as RFC 3339 with milliseconds and offset
/// (`2026-11-03T19:55:39.123+11:00`), the timestamp on every log line.
pub fn log_timestamp() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
}

pub fn set_persistence_enabled(enabled: bool) {
    let was_enabled = PERSISTENCE_ENABLED.swap(enabled, Ordering::SeqCst);
    if enabled && !was_enabled && SESSION_LOG_FILE.get().is_none() {
        initialize_session_logging_inner();
    }
}

pub fn get_session_log_path() -> Result<PathBuf, String> {
    Ok(crate::paths::debug_dir()?.join("session.log"))
}

pub fn get_app_config_root_dir() -> Result<PathBuf, String> {
    crate::paths::ensure_app_root()
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
                    log_timestamp(),
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
    if !PERSISTENCE_ENABLED.load(Ordering::SeqCst) {
        return;
    }
    initialize_session_logging_inner();
}

fn initialize_session_logging_inner() {
    let log_path = match get_session_log_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to initialize session log path: {}", error);
            return;
        }
    };

    // Rotate previous session log so it survives for post-crash diagnosis
    if log_path.exists() {
        let last_path = log_path.with_file_name("last-session.log");
        let _ = fs::rename(&log_path, &last_path);
    }

    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
    {
        Ok(mut file) => {
            // Build profile and executable path tell a dev build apart from an
            // installed package of the same version.
            let build_profile = if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            };
            let executable = std::env::current_exe()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|error| format!("<unknown: {error}>"));
            let startup_header = format!(
                "[{}] SESSION START | version={} | build={} | exe={}\n",
                log_timestamp(),
                env!("CARGO_PKG_VERSION"),
                build_profile,
                executable
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

/// Records every panic in the session log (then runs the default hook), so an
/// unexpected exit always leaves its reason and location behind.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .map(|message| message.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "non-text panic payload".to_string());
        let location = info
            .location()
            .map(|location| format!("{}:{}", location.file(), location.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        let thread = std::thread::current();
        append_session_log(
            "PANIC",
            &log_timestamp(),
            &format!(
                "thread '{}' panicked at {}: {}",
                thread.name().unwrap_or("unnamed"),
                location,
                message
            ),
        );
        default_hook(info);
    }));
}

pub fn append_session_log(level: &str, timestamp: &str, message: &str) {
    if !PERSISTENCE_ENABLED.load(Ordering::Relaxed) {
        return;
    }
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

#[cfg(test)]
mod tests {
    use super::log_timestamp;

    #[test]
    fn log_timestamp_is_rfc3339_with_milliseconds_and_offset() {
        let timestamp = log_timestamp();
        let parsed = chrono::DateTime::parse_from_rfc3339(&timestamp)
            .unwrap_or_else(|error| panic!("{timestamp} is not RFC 3339: {error}"));
        assert_eq!(
            parsed.to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
            timestamp
        );
        let fraction = timestamp
            .split('.')
            .nth(1)
            .unwrap_or_else(|| panic!("{timestamp} has no fractional seconds"));
        assert!(fraction[..3]
            .chars()
            .all(|character| character.is_ascii_digit()));
        assert!(
            !timestamp.ends_with('Z'),
            "{timestamp} should carry a numeric offset"
        );
    }
}
