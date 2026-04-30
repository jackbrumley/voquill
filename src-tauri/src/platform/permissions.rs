use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct LinuxPermissions {
    pub audio: bool,
    pub shortcuts: bool,
    pub input_emulation: bool,
    pub shortcuts_status: String,
    pub shortcuts_detail: Option<String>,
    pub manual_overlay_offset_supported: bool,
    pub overlay_positioning_detail: Option<String>,
}

#[cfg(not(target_os = "linux"))]
pub async fn check_linux_permissions(_config: &crate::config::Config) -> LinuxPermissions {
    LinuxPermissions {
        audio: true,
        shortcuts: true,
        input_emulation: true,
        shortcuts_status: "ready".to_string(),
        shortcuts_detail: None,
        manual_overlay_offset_supported: true,
        overlay_positioning_detail: None,
    }
}

#[cfg(not(target_os = "linux"))]
pub async fn request_linux_permissions(_app_handle: tauri::AppHandle) -> Result<(), String> {
    Ok(())
}
