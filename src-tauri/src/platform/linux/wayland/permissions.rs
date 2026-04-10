use crate::config::Config;
use crate::platform::linux::wayland::input;
use crate::platform::linux::wayland::portal::capabilities::collect_global_shortcuts_diagnostics;
use crate::platform::permissions::LinuxPermissions;
use ashpd::desktop::camera::Camera;
use tauri::AppHandle;

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

    // 2. Request Input Emulation via Wayland Remote Desktop Portal
    input::establish_input_session(&app_handle, true).await?;

    Ok(())
}
