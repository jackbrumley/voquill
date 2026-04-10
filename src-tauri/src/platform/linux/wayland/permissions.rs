use crate::config::Config;
use crate::platform::linux::wayland::portal::capabilities::collect_global_shortcuts_diagnostics;
use crate::platform::permissions::LinuxPermissions;
use crate::AppState;
use ashpd::desktop::camera::Camera;
use ashpd::desktop::remote_desktop::{DeviceType, RemoteDesktop};
use ashpd::desktop::PersistMode;
use tauri::{AppHandle, Manager};

pub async fn check_linux_permissions(config: &Config) -> LinuxPermissions {
    let audio = Camera::new().await.is_ok();
    let shortcuts_diagnostics = collect_global_shortcuts_diagnostics().await;

    let shortcuts = shortcuts_diagnostics.has_record_shortcut;

    let input_emulation = config.input_token.is_some();

    LinuxPermissions {
        audio,
        shortcuts,
        input_emulation,
        shortcuts_status: if shortcuts {
            "bound".to_string()
        } else if shortcuts_diagnostics.available {
            "unbound".to_string()
        } else {
            "portal-unavailable".to_string()
        },
        shortcuts_detail: shortcuts_diagnostics.detail,
    }
}

pub async fn request_linux_permissions(app_handle: AppHandle) -> Result<(), String> {
    // 1. Request Audio (Mic) via Camera Portal
    let camera = Camera::new().await.map_err(|e| {
        format!(
            "Audio Portal not available: {}. Is xdg-desktop-portal-gtk/kde installed?",
            e
        )
    })?;
    camera
        .request_access()
        .await
        .map_err(|e| format!("Audio access denied: {}", e))?;

    // 2. Request Input Emulation via Remote Desktop Portal
    let remote_desktop = RemoteDesktop::new()
        .await
        .map_err(|e| format!("Remote Desktop Portal not available: {}", e))?;
    let rd_session = remote_desktop
        .create_session()
        .await
        .map_err(|e| format!("Failed to create remote desktop session: {}", e))?;

    // First, we must call select_devices and WAIT for the request to resolve
    let select_request = remote_desktop
        .select_devices(
            &rd_session,
            DeviceType::Keyboard.into(),
            None,
            PersistMode::DoNot,
        )
        .await
        .map_err(|e| format!("Failed to select devices: {}", e))?;

    // Wait for the user to interact with the "Select Devices" dialog
    select_request
        .response()
        .map_err(|e| format!("Device selection cancelled: {}", e))?;

    // Now trigger the actual session start - THIS shows the final OS prompt
    let start_request = remote_desktop
        .start(&rd_session, None)
        .await
        .map_err(|e| format!("Failed to start remote desktop session: {}", e))?;

    // Extract the REAL restore token from the OS response
    let selected_devices = start_request
        .response()
        .map_err(|e| format!("Input emulation request cancelled or denied: {}", e))?;

    // If the OS provides a restore token, use it. Otherwise, use a placeholder.
    // This signals to the UI that the user has completed setup, even if persistence isn't available.
    let i_token = selected_devices
        .restore_token()
        .map(|t| t.to_string())
        .or(Some("session".to_string()));

    {
        let state = app_handle.state::<AppState>();
        let mut config = state.config.lock().unwrap();
        config.input_token = i_token;
        let _ = crate::config::save_config(&config);
    }

    Ok(())
}
