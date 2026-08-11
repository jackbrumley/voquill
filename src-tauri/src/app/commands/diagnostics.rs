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

#[tauri::command]
pub async fn open_debug_folder() -> Result<(), String> {
    crate::log_info!("Tauri Command: open_debug_folder invoked");
    let path = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("foss-voquill")
        .join("debug");

    crate::log_info!("Target debug path: {:?}", path);

    if !path.exists() {
        crate::log_info!("Creating debug directory...");
        std::fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        crate::log_info!("Executing: xdg-open {:?}", path);
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|error| {
                crate::log_info!("Failed to execute xdg-open: {}", error);
                error.to_string()
            })?;
    }
    #[cfg(target_os = "windows")]
    {
        crate::log_info!("Executing: explorer {:?}", path);
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|error| {
                crate::log_info!("Failed to execute explorer: {}", error);
                error.to_string()
            })?;
    }
    #[cfg(target_os = "macos")]
    {
        crate::log_info!("Executing: open {:?}", path);
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|error| {
                crate::log_info!("Failed to execute open: {}", error);
                error.to_string()
            })?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_session_log_text() -> Result<String, String> {
    let log_path = crate::resolve_session_log_path()?;
    std::fs::read_to_string(&log_path).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn copy_session_log_to_clipboard() -> Result<(), String> {
    let logs = get_session_log_text().await?;
    typing::copy_to_clipboard(&logs).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clear_recording_logs() -> Result<u32, String> {
    crate::log_info!("Tauri Command: clear_recording_logs invoked");
    let debug_dir = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("foss-voquill")
        .join("debug");
    let recordings_dir = debug_dir.join("recordings");

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
                    .is_some_and(|s| s.starts_with("recording_"))
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

    crate::log_info!("Deleted {} recording file(s)", total);
    Ok(total)
}

#[tauri::command]
pub async fn open_session_log() -> Result<(), String> {
    let log_path = crate::resolve_session_log_path()?;

    #[cfg(target_os = "linux")]
    {
        crate::log_info!("Executing: xdg-open {:?}", log_path);
        std::process::Command::new("xdg-open")
            .arg(&log_path)
            .spawn()
            .map_err(|error| {
                crate::log_info!("Failed to execute xdg-open for session log: {}", error);
                error.to_string()
            })?;
    }

    #[cfg(target_os = "windows")]
    {
        crate::log_info!("Executing: explorer {:?}", log_path);
        std::process::Command::new("explorer")
            .arg(&log_path)
            .spawn()
            .map_err(|error| {
                crate::log_info!("Failed to execute explorer for session log: {}", error);
                error.to_string()
            })?;
    }

    #[cfg(target_os = "macos")]
    {
        crate::log_info!("Executing: open {:?}", log_path);
        std::process::Command::new("open")
            .arg(&log_path)
            .spawn()
            .map_err(|error| {
                crate::log_info!("Failed to execute open for session log: {}", error);
                error.to_string()
            })?;
    }

    Ok(())
}
