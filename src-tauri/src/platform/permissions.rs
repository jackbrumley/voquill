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
