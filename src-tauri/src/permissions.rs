use serde::Serialize;
use ashpd::desktop::camera::Camera;
#[cfg(target_os = "linux")]
use ashpd::desktop::remote_desktop::{RemoteDesktop, DeviceType};
#[cfg(target_os = "linux")]
use ashpd::desktop::PersistMode;
use tauri::AppHandle;

#[derive(Serialize, Clone, Debug)]
pub struct LinuxPermissions {
    pub audio: bool,
    pub shortcuts: bool,
    pub input_emulation: bool,
}

#[cfg(target_os = "linux")]
pub async fn check_linux_permissions(config: &crate::config::Config) -> LinuxPermissions {
    // Modern Wayland: We check if tokens exist as a proxy for "user has gone through setup"
    // The actual enforcement happens at runtime when we try to use the portals
    
    let audio = Camera::new().await.is_ok();
    
    // For Shortcuts and Input, we check if tokens exist
    // If the user cancels the prompts, these will be None
    let shortcuts = config.shortcuts_token.is_some();
    let input_emulation = config.input_token.is_some();

    LinuxPermissions {
        audio,
        shortcuts,
        input_emulation,
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn check_linux_permissions(_config: &crate::config::Config) -> LinuxPermissions {
    LinuxPermissions {
        audio: true,
        shortcuts: true,
        input_emulation: true,
    }
}

#[cfg(target_os = "linux")]
pub async fn request_linux_permissions(app_handle: AppHandle) -> Result<(), String> {
    use crate::AppState;
    use tauri::Manager;

    // 1. Request Audio (Mic) via Camera Portal
    let camera = Camera::new().await.map_err(|e| format!("Audio Portal not available: {}. Is xdg-desktop-portal-gtk/kde installed?", e))?;
    camera.request_access().await.map_err(|e| format!("Audio access denied: {}", e))?;

    // 2. Request Input Emulation via Remote Desktop Portal
    let remote_desktop = RemoteDesktop::new().await.map_err(|e| format!("Remote Desktop Portal not available: {}", e))?;
    let rd_session = remote_desktop.create_session().await.map_err(|e| format!("Failed to create remote desktop session: {}", e))?;
    
    // First, we must call select_devices and WAIT for the request to resolve
    let select_request = remote_desktop.select_devices(&rd_session, DeviceType::Keyboard.into(), None, PersistMode::DoNot).await.map_err(|e| format!("Failed to select devices: {}", e))?;
    
    // Wait for the user to interact with the "Select Devices" dialog
    select_request.response().map_err(|e| format!("Device selection cancelled: {}", e))?;
    
    // Now trigger the actual session start - THIS shows the final OS prompt
    let start_request = remote_desktop.start(&rd_session, None).await.map_err(|e| format!("Failed to start remote desktop session: {}", e))?;
    
    // Extract the REAL restore token from the OS response
    let selected_devices = start_request.response().map_err(|e| format!("Input emulation request cancelled or denied: {}", e))?;
    
    // If the OS provides a restore token, use it. Otherwise, use a placeholder.
    // This signals to the UI that the user has completed setup, even if persistence isn't available.
    let i_token = selected_devices.restore_token()
        .map(|t| t.to_string())
        .or(Some("session".to_string()));

    // Save tokens to config ONLY if we got here (meaning user granted everything)
    {
        let state = app_handle.state::<AppState>();
        let mut config = state.config.lock().unwrap();
        config.input_token = i_token;
        let _ = crate::config::save_config(&config);
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub async fn request_linux_permissions(_app_handle: AppHandle) -> Result<(), String> {
    Ok(())
}
