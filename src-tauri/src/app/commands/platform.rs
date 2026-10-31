#[cfg(target_os = "linux")]
use crate::platform::linux::detection::is_wayland_session;
use crate::platform::permissions::PlatformPermissions;
use crate::{audio, AppState};
use serde::Serialize;

#[tauri::command]
pub async fn get_wayland_portal_version() -> Result<u32, String> {
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            return Ok(
                crate::platform::linux::wayland::portal::capabilities::detect_global_shortcuts_capabilities()
                    .await
                    .map(|capabilities| capabilities.version)
                    .unwrap_or(0),
            );
        }
    }
    Ok(0)
}

#[cfg(target_os = "linux")]
fn read_linux_distribution_name() -> Option<String> {
    let contents = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return Some(value.trim_matches('"').to_string());
        }
    }
    None
}

#[derive(Serialize)]
pub struct SystemShortcutContext {
    distro: Option<String>,
    desktop: Option<String>,
    settings_path: String,
}

#[derive(Serialize)]
pub struct InputCapabilities {
    paste_shortcuts: Vec<crate::config::PasteShortcut>,
    fn_modifier_supported: bool,
}

/// Input features the active display backend supports, so the UI offers
/// only options that will work on this platform.
#[tauri::command]
pub async fn get_input_capabilities(
    state: tauri::State<'_, AppState>,
) -> Result<InputCapabilities, String> {
    Ok(InputCapabilities {
        paste_shortcuts: state.display_backend.supported_paste_shortcuts().to_vec(),
        fn_modifier_supported: state.display_backend.supports_fn_modifier(),
    })
}

#[derive(Serialize)]
pub struct OverlayPositioningCapabilities {
    manual_offset_supported: bool,
    detail: Option<String>,
}

#[tauri::command]
pub async fn get_overlay_positioning_capabilities() -> Result<OverlayPositioningCapabilities, String>
{
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            let layer_shell_supported =
                crate::platform::linux::wayland::overlay::manual_overlay_offset_supported();
            crate::log_info!(
                "Overlay positioning capability check: wayland=true, manual_offset_supported={}",
                layer_shell_supported
            );
            if !layer_shell_supported {
                return Ok(OverlayPositioningCapabilities {
                    manual_offset_supported: false,
                    detail: Some(
                        "Manual overlay position adjustment is not available on your system."
                            .to_string(),
                    ),
                });
            }
        }
    }

    Ok(OverlayPositioningCapabilities {
        manual_offset_supported: true,
        detail: None,
    })
}

#[tauri::command]
pub async fn get_system_shortcut_context() -> Result<SystemShortcutContext, String> {
    #[cfg(target_os = "linux")]
    {
        let distro = read_linux_distribution_name();
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .ok()
            .and_then(|value| value.split(':').next().map(|segment| segment.to_string()));

        let settings_path = match desktop.as_deref() {
            Some(value) if value.eq_ignore_ascii_case("GNOME") => {
                "System Settings -> Apps -> Voquill -> Global Shortcuts".to_string()
            }
            Some(value) if value.eq_ignore_ascii_case("KDE") => {
                "System Settings -> Keyboard -> Shortcuts -> Voquill".to_string()
            }
            _ => "System Settings -> search for 'Voquill' or 'Keyboard Shortcuts'".to_string(),
        };

        Ok(SystemShortcutContext {
            distro,
            desktop,
            settings_path,
        })
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(SystemShortcutContext {
            distro: None,
            desktop: None,
            settings_path: "System Settings -> Keyboard Shortcuts".to_string(),
        })
    }
}

#[tauri::command]
pub async fn get_platform_setup_status(
    state: tauri::State<'_, AppState>,
) -> Result<PlatformPermissions, String> {
    crate::log_info!("Tauri Command: get_platform_setup_status invoked");
    let config = {
        let guard = state.config.lock().unwrap();
        guard.clone()
    };
    let mut permissions = state.display_backend.check_permissions(&config).await;
    let binding_state = state.hotkey_binding_state.lock().unwrap().clone();
    if binding_state.bound {
        permissions.shortcuts = true;
        permissions.shortcuts_status = "bound".to_string();
        permissions.shortcuts_detail = binding_state.detail;
    }
    #[cfg(target_os = "linux")]
    if is_wayland_session() {
        use crate::platform::linux::wayland::input::InputSessionState;
        let input_session = *state.wayland_input_session.lock().unwrap();
        permissions.input_emulation = input_session == InputSessionState::Ready;
        permissions.input_emulation_restoring = input_session == InputSessionState::Restoring;

        let manual_overlay_offset_supported =
            crate::platform::linux::wayland::overlay::manual_overlay_offset_supported();
        permissions.manual_overlay_offset_supported = manual_overlay_offset_supported;
        permissions.overlay_positioning_detail = if manual_overlay_offset_supported {
            None
        } else {
            Some("Manual overlay position adjustment is not available on your system.".to_string())
        };
    }
    #[cfg(target_os = "macos")]
    {
        // The microphone TCC prompt fires when the persistent engine opens
        // its input stream, so a live engine is the grant signal.
        permissions.audio = state.audio_engine.lock().unwrap().is_some();
    }
    crate::log_info!(
        "Setup readiness: audio={}, shortcuts={} (status={}), input_emulation={}, input_emulation_restoring={}, runtime_hotkey_bound={}, runtime_hotkey_listening={}",
        permissions.audio,
        permissions.shortcuts,
        permissions.shortcuts_status,
        permissions.input_emulation,
        permissions.input_emulation_restoring,
        binding_state.bound,
        binding_state.listening
    );
    Ok(permissions)
}

#[tauri::command]
pub async fn request_audio_permission(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::log_info!("Tauri Command: request_audio_permission invoked");
    state
        .display_backend
        .request_audio_permission(app_handle)
        .await
}

#[tauri::command]
pub async fn request_input_permission(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::log_info!("Tauri Command: request_input_permission invoked");
    state
        .display_backend
        .request_input_permission(app_handle)
        .await
}

#[tauri::command]
pub async fn log_ui_event(message: String) {
    crate::log_info!("[UI] {}", message);
}

#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<audio::AudioDevice>, String> {
    crate::log_info!("Tauri Command: get_audio_devices invoked");
    audio::get_input_devices()
}

#[tauri::command]
pub async fn get_output_devices() -> Result<Vec<audio::AudioDevice>, String> {
    crate::log_info!("Tauri Command: get_output_devices invoked");
    audio::get_output_devices()
}
