use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct LinuxPermissions {
    pub audio: bool,
    pub shortcuts: bool,
    pub input_emulation: bool,
}

#[cfg(not(target_os = "linux"))]
pub async fn check_linux_permissions(_config: &crate::config::Config) -> LinuxPermissions {
    LinuxPermissions {
        audio: true,
        shortcuts: true,
        input_emulation: true,
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn request_linux_permissions(_app_handle: tauri::AppHandle) -> Result<(), String> {
    Ok(())
}
