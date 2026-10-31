use crate::config::Config;
use crate::platform::permissions::PlatformPermissions;
use ashpd::desktop::camera::Camera;

pub async fn check_linux_permissions(config: &Config) -> PlatformPermissions {
    let audio = Camera::new().await.is_ok();
    let shortcuts = config.shortcuts_token.is_some();

    let input_emulation = config.input_token.is_some();

    PlatformPermissions {
        audio,
        shortcuts,
        input_emulation,
        input_emulation_restoring: false,
        shortcuts_status: if shortcuts {
            "bound".to_string()
        } else {
            "unbound".to_string()
        },
        shortcuts_detail: None,
        manual_overlay_offset_supported: false,
        overlay_positioning_detail: Some(
            "Manual overlay position adjustment is not available on your system.".to_string(),
        ),
    }
}

/// Requests device access through the portal's Camera interface, which is
/// what gates the PipeWire device for the microphone on Wayland.
pub async fn request_microphone_access() -> Result<(), String> {
    let camera = Camera::new().await.map_err(|error| {
        format!(
            "Audio Portal not available: {}. Is xdg-desktop-portal installed?",
            error
        )
    })?;
    camera
        .request_access()
        .await
        .map_err(|error| format!("Audio access denied: {}", error))?;
    Ok(())
}
