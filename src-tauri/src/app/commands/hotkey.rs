use crate::{AppState, HotkeyBindingState};
use serde::Serialize;
use tauri::{Emitter, Manager};

#[cfg(target_os = "linux")]
use crate::platform::linux::detection::is_wayland_session;

#[cfg(target_os = "linux")]
fn enrich_wayland_shortcut_error(state: &AppState, error: String) -> String {
    if !is_wayland_session() {
        return error;
    }

    let looks_like_portal_rejection = error.contains("Portal request didn't succeed: Other")
        || error.contains("Portal rejected shortcut");
    if !looks_like_portal_rejection {
        return error;
    }

    let host_registration_error = state
        .wayland_host_app_registration_error
        .lock()
        .unwrap()
        .clone();

    if let Some(host_error) = host_registration_error {
        return format!(
            "{} Hint: Wayland portal app registration failed earlier ({}). This usually means the desktop environment cannot resolve Voquill's app metadata yet.",
            error, host_error
        );
    }

    error
}

pub fn set_hotkey_binding_state(
    app_handle: &tauri::AppHandle,
    bound: bool,
    listening: bool,
    detail: Option<String>,
    active_trigger: Option<String>,
) {
    let state = app_handle.state::<AppState>();
    {
        let mut binding_state = state.hotkey_binding_state.lock().unwrap();
        binding_state.bound = bound;
        binding_state.listening = listening;
        binding_state.detail = detail;
        binding_state.active_trigger = active_trigger;
    }
    let snapshot = {
        let binding_state = state.hotkey_binding_state.lock().unwrap();
        binding_state.clone()
    };
    let _ = app_handle.emit("hotkey-binding-state", snapshot);
}

#[derive(Serialize)]
pub struct ConfigureHotkeyResult {
    outcome: String,
    detail: Option<String>,
}

async fn apply_hotkey_registration(
    new_hotkey: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    crate::log_info!("Manual hotkey registration requested for: {}", new_hotkey);

    let previous_hotkey = {
        let config = state.config.lock().unwrap();
        config.hotkey.clone()
    };

    {
        let mut config = state.config.lock().unwrap();
        config.hotkey = new_hotkey.clone();
    }

    let backend = state.display_backend.clone();
    match backend.start_engine(app_handle.clone(), true).await {
        Ok(()) => {
            let save_result = {
                let config = state.config.lock().unwrap();
                crate::config::save_config(&config)
            };

            let save_error = save_result.err().map(|error| error.to_string());
            if let Some(save_error) = save_error {
                crate::log_warn!(
                    "Failed to persist new hotkey '{}': {}. Restoring previous hotkey '{}'.",
                    new_hotkey,
                    save_error,
                    previous_hotkey
                );

                {
                    let mut config = state.config.lock().unwrap();
                    config.hotkey = previous_hotkey.clone();
                }

                let restore_error = backend.start_engine(app_handle.clone(), false).await.err();
                if let Some(error) = restore_error {
                    let enriched_error = {
                        #[cfg(target_os = "linux")]
                        {
                            enrich_wayland_shortcut_error(&state, error)
                        }
                        #[cfg(not(target_os = "linux"))]
                        {
                            error
                        }
                    };
                    set_hotkey_binding_state(
                        &app_handle,
                        false,
                        false,
                        Some(enriched_error.clone()),
                        None,
                    );
                    let mut hotkey_error = state.hotkey_error.lock().unwrap();
                    *hotkey_error = Some(enriched_error.clone());
                    return Err(format!(
                        "Failed to save hotkey change: {}. Also failed to restore previous hotkey: {}",
                        save_error, enriched_error
                    ));
                }

                set_hotkey_binding_state(&app_handle, true, true, None, None);
                let mut hotkey_error = state.hotkey_error.lock().unwrap();
                *hotkey_error = None;
                return Err(format!(
                    "Failed to save hotkey change: {}. Previous hotkey was restored.",
                    save_error
                ));
            }

            set_hotkey_binding_state(&app_handle, true, true, None, None);
            let mut error = state.hotkey_error.lock().unwrap();
            *error = None;
            Ok(())
        }
        Err(error) => {
            let registration_error = {
                #[cfg(target_os = "linux")]
                {
                    enrich_wayland_shortcut_error(&state, error)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    error
                }
            };

            {
                let mut config = state.config.lock().unwrap();
                config.hotkey = previous_hotkey.clone();
            }

            if previous_hotkey == new_hotkey {
                set_hotkey_binding_state(
                    &app_handle,
                    false,
                    false,
                    Some(registration_error.clone()),
                    None,
                );
                let mut hotkey_error = state.hotkey_error.lock().unwrap();
                *hotkey_error = Some(registration_error.clone());
                return Err(registration_error);
            }

            let restore_result = backend.start_engine(app_handle.clone(), false).await;
            match restore_result {
                Ok(()) => {
                    set_hotkey_binding_state(&app_handle, true, true, None, None);
                    let mut hotkey_error = state.hotkey_error.lock().unwrap();
                    *hotkey_error = None;
                    Err(registration_error)
                }
                Err(restore_error) => {
                    let enriched_restore_error = {
                        #[cfg(target_os = "linux")]
                        {
                            enrich_wayland_shortcut_error(&state, restore_error)
                        }
                        #[cfg(not(target_os = "linux"))]
                        {
                            restore_error
                        }
                    };
                    set_hotkey_binding_state(
                        &app_handle,
                        false,
                        false,
                        Some(enriched_restore_error.clone()),
                        None,
                    );
                    let mut hotkey_error = state.hotkey_error.lock().unwrap();
                    *hotkey_error = Some(enriched_restore_error.clone());
                    Err(format!(
                        "{} Also failed to restore previous hotkey: {}",
                        registration_error, enriched_restore_error
                    ))
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[tauri::command]
pub async fn configure_hotkey(
    state: tauri::State<'_, AppState>,
) -> Result<ConfigureHotkeyResult, String> {
    if is_wayland_session() {
        let capabilities =
            crate::platform::linux::wayland::portal::capabilities::detect_global_shortcuts_capabilities()
                .await?;

        if !capabilities.supports_configure_shortcuts {
            return Ok(ConfigureHotkeyResult {
                outcome: "system_managed".to_string(),
                detail: Some(
                    "This desktop manages shortcut changes in system settings.".to_string(),
                ),
            });
        }

        let hotkey = {
            let config = state.config.lock().unwrap();
            config.hotkey.clone()
        };

        let opened_system_configuration =
            crate::platform::linux::wayland::shortcuts::try_open_linux_portal_shortcut_configuration(
                &hotkey,
            )
            .await?;

        if opened_system_configuration {
            return Ok(ConfigureHotkeyResult {
                outcome: "configured".to_string(),
                detail: Some("Opened system shortcut configuration.".to_string()),
            });
        }

        return Ok(ConfigureHotkeyResult {
            outcome: "system_managed".to_string(),
            detail: Some("Shortcut changes must be made in system settings.".to_string()),
        });
    }

    Ok(ConfigureHotkeyResult {
        outcome: "requires_in_app_capture".to_string(),
        detail: None,
    })
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
pub async fn configure_hotkey() -> Result<ConfigureHotkeyResult, String> {
    Ok(ConfigureHotkeyResult {
        outcome: "requires_in_app_capture".to_string(),
        detail: None,
    })
}

#[tauri::command]
pub async fn apply_captured_hotkey(
    new_hotkey: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<ConfigureHotkeyResult, String> {
    apply_hotkey_registration(new_hotkey, state, app_handle).await?;
    Ok(ConfigureHotkeyResult {
        outcome: "configured".to_string(),
        detail: None,
    })
}

#[tauri::command]
pub async fn manual_register_hotkey(
    new_hotkey: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    apply_hotkey_registration(new_hotkey, state, app_handle).await
}

#[tauri::command]
pub async fn check_hotkey_status(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let error = state.hotkey_error.lock().unwrap();
    Ok(error.clone())
}

#[tauri::command]
pub async fn get_hotkey_binding_state(
    state: tauri::State<'_, AppState>,
) -> Result<HotkeyBindingState, String> {
    let binding_state = state.hotkey_binding_state.lock().unwrap();
    Ok(binding_state.clone())
}

#[cfg(target_os = "linux")]
#[tauri::command]
pub async fn set_configuring_hotkey(
    is_configuring: bool,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut config_flag = state.is_configuring_hotkey.lock().unwrap();
        *config_flag = is_configuring;
    }
    crate::log_info!("🔧 set_configuring_hotkey: {}", is_configuring);

    if is_wayland_session() {
        if is_configuring {
            let cancelled = {
                let mut cancel_lock = state.hotkey_engine_cancel.lock().unwrap();
                if let Some(sender) = cancel_lock.take() {
                    let _ = sender.send(());
                    true
                } else {
                    false
                }
            };
            if cancelled {
                crate::log_info!("⏸️  Paused Wayland hotkey engine while configuring shortcut");
            }
        } else {
            let should_resume = {
                let binding_state = state.hotkey_binding_state.lock().unwrap();
                !(binding_state.bound && binding_state.listening)
            };

            if should_resume {
                let backend = state.display_backend.clone();
                if let Err(error) = backend.start_engine(app_handle.clone(), false).await {
                    crate::log_warn!(
                        "Failed to resume Wayland hotkey engine after configuration: {}",
                        error
                    );
                }
            } else {
                crate::log_info!("▶️ Wayland hotkey engine already active after capture; skipping resume");
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
pub async fn set_configuring_hotkey(
    is_configuring: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut config_flag = state.is_configuring_hotkey.lock().unwrap();
        *config_flag = is_configuring;
    }
    crate::log_info!("🔧 set_configuring_hotkey: {}", is_configuring);
    Ok(())
}

pub async fn re_register_hotkey(
    app_handle: &tauri::AppHandle,
    _hotkey_string: &str,
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();

    let backend = state.display_backend.clone();
    match backend.start_engine(app_handle.clone(), false).await {
        Ok(()) => {
            set_hotkey_binding_state(app_handle, true, true, None, None);
            Ok(())
        }
        Err(error) => {
            set_hotkey_binding_state(app_handle, false, false, Some(error.clone()), None);
            Err(error)
        }
    }
}
