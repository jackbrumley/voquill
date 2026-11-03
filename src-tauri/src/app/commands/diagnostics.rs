use crate::app::system_info::SystemInfo;
use crate::AppState;
use serde::Serialize;

#[derive(Serialize)]
pub struct GpuStatus {
    pub tested: bool,
    pub available: bool,
    pub detail: Option<String>,
}

#[tauri::command]
pub async fn get_gpu_status(state: tauri::State<'_, AppState>) -> Result<GpuStatus, String> {
    let factory = &state.engine_factory;
    let tested = factory.gpu_has_been_tested();
    if tested {
        match factory.last_gpu_error() {
            None => Ok(GpuStatus {
                tested: true,
                available: true,
                detail: None,
            }),
            Some(error) => Ok(GpuStatus {
                tested: true,
                available: false,
                detail: Some(error),
            }),
        }
    } else {
        Ok(GpuStatus {
            tested: false,
            available: false,
            detail: None,
        })
    }
}

#[tauri::command]
pub async fn get_post_process_gpu_status(
    state: tauri::State<'_, AppState>,
) -> Result<GpuStatus, String> {
    let factory = &state.post_process_factory;
    let tested = factory.gpu_has_been_tested();
    if tested {
        match factory.last_gpu_error() {
            None => Ok(GpuStatus {
                tested: true,
                available: true,
                detail: None,
            }),
            Some(error) => Ok(GpuStatus {
                tested: true,
                available: false,
                detail: Some(error),
            }),
        }
    } else {
        Ok(GpuStatus {
            tested: false,
            available: false,
            detail: None,
        })
    }
}

use crate::typing;

/// Opens a file or folder with the desktop's default handler.
pub(crate) fn open_with_system(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let opener = "xdg-open";
    #[cfg(target_os = "windows")]
    let opener = "explorer";
    #[cfg(target_os = "macos")]
    let opener = "open";

    crate::log_info!("Executing: {} {:?}", opener, path);
    std::process::Command::new(opener)
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| {
            crate::log_warn!("Failed to execute {} for {:?}: {}", opener, path, error);
            error.to_string()
        })
}

/// Opens the debug folder (session log, last-session log, update log and
/// recording logs).
#[tauri::command]
pub async fn open_log_folder() -> Result<(), String> {
    crate::log_info!("Tauri Command: open_log_folder invoked");
    open_with_system(&crate::paths::debug_dir()?)
}

#[tauri::command]
pub async fn copy_session_log() -> Result<(), String> {
    let log_path = crate::resolve_session_log_path()?;
    let log_text = std::fs::read_to_string(&log_path).map_err(|error| error.to_string())?;
    typing::copy_to_clipboard(&log_text).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo::collect())
}

/// The System info block's copy button, for bug reports.
#[tauri::command]
pub async fn copy_system_info() -> Result<(), String> {
    typing::copy_to_clipboard(&SystemInfo::collect().summary()).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clear_recording_logs() -> Result<u32, String> {
    crate::log_info!("Tauri Command: clear_recording_logs invoked");
    let debug_dir = crate::paths::debug_dir()?;
    let recordings_dir = crate::paths::debug_recordings_dir()?;

    let mut total = 0u32;

    if debug_dir.exists() {
        for entry in std::fs::read_dir(&debug_dir)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let path = entry.path();
            if path.extension().map(|ext| ext == "wav").unwrap_or(false)
                && path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("recording_") || s.starts_with("import_"))
                && std::fs::remove_file(&path).is_ok()
            {
                total += 1;
            }
        }
    }

    if recordings_dir.exists() {
        for entry in std::fs::read_dir(&recordings_dir)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let path = entry.path();
            if path.extension().map(|ext| ext == "wav").unwrap_or(false)
                && std::fs::remove_file(&path).is_ok()
            {
                total += 1;
            }
        }
    }

    let _ = crate::history::clear_audio_file_references();

    crate::log_info!("Deleted {} recording file(s)", total);
    Ok(total)
}

#[tauri::command]
pub async fn open_session_log() -> Result<(), String> {
    open_with_system(&crate::resolve_session_log_path()?)
}
