use crate::{audio, AppState};
#[cfg(target_os = "linux")]
use crate::platform::linux::wayland::portal::capabilities::PortalDiagnostics;
#[cfg(not(target_os = "linux"))]
use crate::PortalDiagnostics;
use serde::Serialize;
#[cfg(target_os = "linux")]
use crate::platform::linux::detection::is_wayland_session;
use crate::platform::permissions::LinuxPermissions;
use tauri::Manager;

#[tauri::command]
pub async fn get_wayland_portal_version() -> Result<u32, String> {
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            use ashpd::desktop::global_shortcuts::GlobalShortcuts;
            if let Ok(proxy) = GlobalShortcuts::new().await {
                use std::ops::Deref;
                if let Ok(version) = proxy.deref().get_property::<u32>("version").await {
                    return Ok(version);
                }
            }
            return Ok(0);
        }
    }
    Ok(0)
}

#[tauri::command]
pub async fn get_portal_diagnostics() -> Result<PortalDiagnostics, String> {
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            return Ok(
                crate::platform::linux::wayland::portal::capabilities::collect_global_shortcuts_diagnostics().await,
            );
        }
    }

    Ok(PortalDiagnostics {
        available: false,
        version: 0,
        supports_configure_shortcuts: false,
        has_record_shortcut: false,
        active_trigger: None,
        status: "unsupported".to_string(),
        detail: Some("Portal diagnostics are only available on Linux Wayland.".to_string()),
    })
}

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

        return Ok(SystemShortcutContext {
            distro,
            desktop,
            settings_path,
        });
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
pub async fn get_linux_setup_status(
    state: tauri::State<'_, AppState>,
) -> Result<LinuxPermissions, String> {
    crate::log_info!("📡 Tauri Command: get_linux_setup_status invoked");
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
        let input_ready = *state.wayland_input_ready.lock().unwrap();
        permissions.input_emulation = input_ready;
    }
    crate::log_info!(
        "🧭 Setup readiness: audio={}, shortcuts={} (status={}), input_emulation={}, runtime_hotkey_bound={}, runtime_hotkey_listening={}",
        permissions.audio,
        permissions.shortcuts,
        permissions.shortcuts_status,
        permissions.input_emulation,
        binding_state.bound,
        binding_state.listening
    );
    Ok(permissions)
}

#[tauri::command]
pub async fn request_audio_permission() -> Result<(), String> {
    crate::log_info!("📡 Tauri Command: request_audio_permission invoked");
    #[cfg(target_os = "linux")]
    {
        use ashpd::desktop::camera::Camera;

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
        return Ok(());
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(())
    }
}

#[tauri::command]
pub async fn request_input_permission(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::log_info!("📡 Tauri Command: request_input_permission invoked");
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            crate::platform::linux::wayland::input::establish_input_session(&app_handle, true)
                .await?;
        } else {
            let _ = state;
        }
        return Ok(());
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = state;
        let _ = app_handle;
        Ok(())
    }
}

#[tauri::command]
pub async fn log_ui_event(message: String) {
    crate::log_info!("[UI] {}", message);
}

#[cfg(target_os = "linux")]
pub async fn is_status_notifier_watcher_available() -> bool {
    use zbus::names::BusName;

    let connection = match zbus::Connection::session().await {
        Ok(connection) => connection,
        Err(error) => {
            crate::log_warn!("Failed to open session DBus for tray check: {}", error);
            return false;
        }
    };

    let proxy = match zbus::fdo::DBusProxy::new(&connection).await {
        Ok(proxy) => proxy,
        Err(error) => {
            crate::log_warn!("Failed to create DBus proxy for tray check: {}", error);
            return false;
        }
    };

    let kde_watcher = match BusName::try_from("org.kde.StatusNotifierWatcher") {
        Ok(name) => name,
        Err(error) => {
            crate::log_warn!("Invalid KDE watcher bus name: {}", error);
            return false;
        }
    };
    let freedesktop_watcher = match BusName::try_from("org.freedesktop.StatusNotifierWatcher") {
        Ok(name) => name,
        Err(error) => {
            crate::log_warn!("Invalid freedesktop watcher bus name: {}", error);
            return false;
        }
    };

    match proxy.name_has_owner(kde_watcher).await {
        Ok(true) => true,
        Ok(false) => match proxy.name_has_owner(freedesktop_watcher).await {
            Ok(value) => value,
            Err(error) => {
                crate::log_warn!("Failed to check freedesktop tray watcher: {}", error);
                false
            }
        },
        Err(error) => {
            crate::log_warn!("Failed to check KDE tray watcher: {}", error);
            false
        }
    }
}

#[tauri::command]
pub async fn minimize_to_tray_or_taskbar(app_handle: tauri::AppHandle) -> Result<String, String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    #[cfg(target_os = "linux")]
    {
        if is_status_notifier_watcher_available().await {
            window.hide().map_err(|error| error.to_string())?;
            return Ok("tray".to_string());
        }

        window.minimize().map_err(|error| error.to_string())?;
        return Ok("taskbar".to_string());
    }

    #[cfg(not(target_os = "linux"))]
    {
        window.minimize().map_err(|error| error.to_string())?;
        Ok("taskbar".to_string())
    }
}

#[tauri::command]
pub async fn quit_application(app_handle: tauri::AppHandle) -> Result<(), String> {
    app_handle.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<audio::AudioDevice>, String> {
    crate::log_info!("📡 Tauri Command: get_audio_devices invoked");
    audio::get_input_devices()
}
