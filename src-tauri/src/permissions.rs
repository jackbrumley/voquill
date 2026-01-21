use serde::Serialize;
use ashpd::desktop::camera::Camera;
#[cfg(target_os = "linux")]
use ashpd::desktop::global_shortcuts::GlobalShortcuts;
#[cfg(target_os = "linux")]
use ashpd::desktop::remote_desktop::RemoteDesktop;
use std::fs;
use tauri::AppHandle;

#[derive(Serialize, Clone, Debug)]
pub struct LinuxPermissions {
    pub audio: bool,
    pub shortcuts: bool,
    pub input_emulation: bool,
}

#[cfg(target_os = "linux")]
pub async fn check_linux_permissions() -> LinuxPermissions {
    // 1. Audio check - check if we can list devices as a proxy for permissions
    let audio = fs::read_dir("/dev/snd").is_ok();

    // 2. Shortcuts check - can we interact with the GlobalShortcuts portal?
    // This doesn't mean we HAVE a shortcut, just that the portal is available.
    let shortcuts = GlobalShortcuts::new().await.is_ok();

    // 3. Input emulation - check if uinput is writable (legacy) or portal is available
    let has_uinput = fs::OpenOptions::new().write(true).open("/dev/uinput").is_ok();
    let has_portal = RemoteDesktop::new().await.is_ok();
    let input_emulation = has_uinput || has_portal;

    LinuxPermissions {
        audio,
        shortcuts,
        input_emulation,
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn check_linux_permissions() -> LinuxPermissions {
    LinuxPermissions {
        audio: true,
        shortcuts: true,
        input_emulation: true,
    }
}

#[cfg(target_os = "linux")]
pub async fn request_linux_permissions(_app_handle: AppHandle) -> Result<(), String> {
    // 1. Request Audio (via Camera portal proxy which handles Mic too in some versions)
    let camera = Camera::new().await.map_err(|e| format!("Audio Portal not available: {}. Is xdg-desktop-portal-gtk/kde installed?", e))?;
    camera.request_access().await.map_err(|e| format!("Audio access denied: {}", e))?;

    // 2. Request Global Shortcuts
    let shortcuts = GlobalShortcuts::new().await.map_err(|e| format!("Global Shortcuts Portal not available: {}. Your compositor might not support it.", e))?;
    let session = shortcuts.create_session().await.map_err(|e| format!("Failed to create shortcuts session: {}", e))?;
    shortcuts.list_shortcuts(&session).await.map_err(|e| format!("Failed to list/init shortcuts: {}", e))?;

    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub async fn request_linux_permissions(_app_handle: AppHandle) -> Result<(), String> {
    Ok(())
}
