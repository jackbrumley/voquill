use crate::AppState;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static CURRENT_STATUS: OnceLock<Mutex<String>> = OnceLock::new();
static STATUS_UPDATE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize)]
struct StatusUpdatePayload {
    seq: u64,
    status: String,
}

pub fn initialize(app_handle: AppHandle) {
    let _ = APP_HANDLE.set(app_handle);
    let _ = CURRENT_STATUS.set(Mutex::new("Ready".to_string()));
}

pub fn get_current_status() -> String {
    if let Some(status_mutex) = CURRENT_STATUS.get() {
        if let Ok(status) = status_mutex.lock() {
            return status.clone();
        }
    }
    "Ready".to_string()
}

async fn hide_overlay_window(app_handle: &AppHandle) -> Result<(), String> {
    if let Some(overlay_window) = app_handle.get_webview_window("overlay") {
        overlay_window.hide().map_err(|error| error.to_string())?;
    } else {
        crate::log_warn!("⚠️ hide_overlay_window: overlay window not found");
    }
    Ok(())
}

async fn position_overlay_window(
    overlay_window: &WebviewWindow,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let app_state = app_handle.state::<AppState>();
    let pixels_from_bottom_logical = {
        let config = app_state.config.lock().unwrap();
        config.pixels_from_bottom
    };

    app_state
        .display_backend
        .position_overlay_window(overlay_window, pixels_from_bottom_logical)?;
    Ok(())
}

async fn show_overlay_window(app_handle: &AppHandle) -> Result<(), String> {
    let overlay_window = app_handle
        .get_webview_window("overlay")
        .ok_or("Overlay window not found")?;

    if overlay_window.is_visible().unwrap_or(false) {
        return Ok(());
    }

    position_overlay_window(&overlay_window, app_handle).await?;
    overlay_window.show().map_err(|error| error.to_string())?;

    let state = app_handle.state::<AppState>();
    state.display_backend.apply_overlay_hints(&overlay_window);
    Ok(())
}

pub async fn emit_status_update(status: &str) {
    let sequence = STATUS_UPDATE_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut previous_status: Option<String> = None;
    let mut changed = false;
    if let Some(status_mutex) = CURRENT_STATUS.get() {
        if let Ok(mut global_status) = status_mutex.lock() {
            previous_status = Some(global_status.clone());
            if *global_status != status {
                *global_status = status.to_string();
                changed = true;
            }
        }
    }

    if !changed {
        return;
    }

    crate::log_info!(
        "🔄 App Status Change: '{}' -> '{}'",
        previous_status.as_deref().unwrap_or("<unknown>"),
        status
    );

    if let Some(app_handle) = APP_HANDLE.get() {
        let windows = ["main", "overlay"];
        let payload = StatusUpdatePayload {
            seq: sequence,
            status: status.to_string(),
        };
        for window_label in &windows {
            if let Some(window) = app_handle.get_webview_window(window_label) {
                let _ = window.emit("status-update", payload.clone());
            }
        }

        if status == "Ready" || status == "Typing" {
            let _ = hide_overlay_window(app_handle).await;
        } else {
            let _ = show_overlay_window(app_handle).await;
        }
    }
}

pub async fn emit_status_to_frontend(status: &str) {
    emit_status_update(status).await;
}
