use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct PlatformPermissions {
    pub audio: bool,
    pub shortcuts: bool,
    pub input_emulation: bool,
    /// True while a stored portal restore token is being resumed, so the UI
    /// can defer its launch routing instead of treating input as denied.
    pub input_emulation_restoring: bool,
    pub shortcuts_status: String,
    pub shortcuts_detail: Option<String>,
    pub manual_overlay_offset_supported: bool,
    pub overlay_positioning_detail: Option<String>,
}

#[cfg(not(target_os = "linux"))]
pub async fn check_linux_permissions(_config: &crate::config::Config) -> PlatformPermissions {
    PlatformPermissions {
        audio: true,
        shortcuts: true,
        input_emulation: true,
        input_emulation_restoring: false,
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
