use crate::AppState;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;
use tauri::{Emitter, Manager};

use super::capabilities::detect_global_shortcuts_capabilities;
use super::types::GlobalShortcutsFlow;

const RECORD_SHORTCUT_ID: &str = "record";

pub fn normalize_wayland_trigger(hotkey: &str) -> String {
    let mut parts: Vec<String> = hotkey
        .split('+')
        .map(|segment| segment.trim().to_string())
        .collect();
    if parts.is_empty() {
        return hotkey.to_string();
    }
    if parts.len() < 2 {
        return normalize_wayland_key_name(&parts[0]);
    }

    let mut key = parts.pop().unwrap_or_default();
    key = normalize_wayland_key_name(&key);

    let mut modifiers = parts;
    for modifier in &mut modifiers {
        *modifier = modifier.to_uppercase();
    }

    modifiers.join("+") + "+" + &key
}

fn normalize_wayland_key_name(key: &str) -> String {
    if key.eq_ignore_ascii_case("space") {
        "space".to_string()
    } else if key.eq_ignore_ascii_case("enter") || key.eq_ignore_ascii_case("return") {
        "Return".to_string()
    } else if key.len() >= 2
        && (key.starts_with('f') || key.starts_with('F'))
        && key[1..].chars().all(|character| character.is_ascii_digit())
    {
        format!("F{}", &key[1..])
    } else {
        key.to_string()
    }
}

async fn bind_record_shortcut(
    proxy: &GlobalShortcuts<'_>,
    session: &ashpd::desktop::Session<'_, GlobalShortcuts<'_>>,
    normalized_trigger: &str,
) -> Result<String, String> {
    let shortcut = NewShortcut::new(RECORD_SHORTCUT_ID, "Dictation Hotkey")
        .preferred_trigger(Some(normalized_trigger));

    let bind_result = proxy
        .bind_shortcuts(session, &[shortcut], None)
        .await
        .map_err(|error| format!("Failed to call portal BindShortcuts: {error}"))?;

    let bound = bind_result
        .response()
        .map_err(|error| format!("Portal rejected shortcut: {error}"))?;

    crate::log_info!(
        "📋 BindShortcuts returned {} shortcuts",
        bound.shortcuts().len()
    );

    if bound.shortcuts().is_empty() {
        return Err("OS rejected the shortcut request. The hotkey may be in use by another application or the system.".to_string());
    }

    for shortcut in bound.shortcuts() {
        crate::log_info!(
            "✅ Wayland Global Shortcuts bound: ID='{}', Trigger='{}'",
            shortcut.id(),
            shortcut.trigger_description()
        );
        if shortcut.id() == RECORD_SHORTCUT_ID {
            return Ok(shortcut.trigger_description().to_string());
        }
    }

    Err("Portal bind succeeded but did not return the expected shortcut id 'record'.".to_string())
}

pub async fn start_linux_portal_hotkey_engine(
    app_handle: tauri::AppHandle,
    force: bool,
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();

    let has_previous = {
        let mut cancel_lock = state.hotkey_engine_cancel.lock().unwrap();
        if let Some(sender) = cancel_lock.take() {
            crate::log_info!("🔄 Cancelling previous hotkey engine...");
            let _ = sender.send(());
            true
        } else {
            false
        }
    };

    if has_previous {
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }

    let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let mut cancel_lock = state.hotkey_engine_cancel.lock().unwrap();
        *cancel_lock = Some(cancel_tx);
    }

    crate::log_info!("🚀 Wayland Global Shortcuts Engine starting...");

    let capabilities = detect_global_shortcuts_capabilities().await?;
    crate::log_info!(
        "🧭 GlobalShortcuts portal version={}, supports_configure_shortcuts={}",
        capabilities.version,
        capabilities.supports_configure_shortcuts
    );

    let (shortcuts_token, hotkey_str) = {
        let config = state.config.lock().unwrap();
        (config.shortcuts_token.clone(), config.hotkey.clone())
    };

    let proxy = GlobalShortcuts::new()
        .await
        .map_err(|error| format!("Failed to connect to Portal: {error}"))?;

    let session = proxy
        .create_session()
        .await
        .map_err(|error| format!("Failed to create portal session: {error}"))?;

    let normalized_trigger = normalize_wayland_trigger(&hotkey_str);
    let flow = GlobalShortcutsFlow::from_force(force);
    crate::log_info!(
        "🔑 Desired trigger='{}', normalized='{}', flow={}",
        hotkey_str,
        normalized_trigger,
        flow.as_str()
    );

    let mut active_trigger = String::new();

    if matches!(flow, GlobalShortcutsFlow::BindNew) {
        active_trigger =
            bind_record_shortcut(&proxy, &session, normalized_trigger.as_str()).await?;

        if active_trigger.is_empty() {
            let _ = session.close().await;
            crate::set_hotkey_binding_state(
                &app_handle,
                false,
                false,
                Some("Portal bind succeeded but did not return expected shortcut id.".to_string()),
                None,
            );
            return Err(
                "Portal bind succeeded but did not return the expected shortcut id 'record'."
                    .to_string(),
            );
        }

        if !trigger_description_matches_request(&active_trigger, &normalized_trigger) {
            crate::log_warn!(
                "Portal kept existing shortcut '{}' instead of requested '{}'.",
                active_trigger,
                normalized_trigger
            );
            let effective_hotkey = trigger_description_to_hotkey(&active_trigger)
                .unwrap_or_else(|| hotkey_str.clone());
            {
                let mut config = state.config.lock().unwrap();
                config.hotkey = effective_hotkey;
                let _ = crate::config::save_config(&config);
            }
            let _ = app_handle.emit("config-updated", ());
        }
    } else {
        let listed = proxy
            .list_shortcuts(&session)
            .await
            .map_err(|error| format!("Failed to call portal ListShortcuts: {error}"))?
            .response()
            .map_err(|error| format!("Failed to read shortcut list response: {error}"))?;

        crate::log_info!(
            "📋 ListShortcuts returned {} shortcuts",
            listed.shortcuts().len()
        );

        for shortcut in listed.shortcuts() {
            if shortcut.id() == RECORD_SHORTCUT_ID {
                active_trigger = shortcut.trigger_description().to_string();
                break;
            }
        }

        if active_trigger.is_empty() {
            if shortcuts_token.is_some() {
                crate::log_warn!(
                    "No active shortcut reported by ListShortcuts; rebinding using desired trigger '{}'",
                    normalized_trigger
                );
                active_trigger =
                    bind_record_shortcut(&proxy, &session, normalized_trigger.as_str()).await?;
            }
            if active_trigger.is_empty() {
                let _ = session.close().await;
                crate::set_hotkey_binding_state(
                    &app_handle,
                    false,
                    false,
                    Some("No system shortcut found. Setup is required.".to_string()),
                    None,
                );
                return Err("No system shortcut found. Setup is required.".to_string());
            }
        }

        crate::log_info!("✅ Reusing existing portal shortcut: '{}'", active_trigger);
    }

    {
        let mut config = state.config.lock().unwrap();
        config.shortcuts_token = Some("granted".to_string());
        let _ = crate::config::save_config(&config);
    }
    crate::set_hotkey_binding_state(&app_handle, true, true, None, Some(active_trigger.clone()));
    let _ = app_handle.emit("config-updated", ());

    let mut activated_stream = proxy
        .receive_activated()
        .await
        .map_err(|error| format!("Failed to listen for shortcut activation: {error}"))?;

    let mut deactivated_stream = proxy
        .receive_deactivated()
        .await
        .map_err(|error| format!("Failed to listen for shortcut deactivation: {error}"))?;

    let app_handle_for_task = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        crate::log_info!("👂 Listening for shortcut events...");
        let mut shortcut_pressed = false;
        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    crate::log_info!("🛑 Hotkey engine cancelled.");
                    crate::set_hotkey_binding_state(
                        &app_handle_for_task,
                        false,
                        false,
                        Some("Hotkey engine stopped.".to_string()),
                        None,
                    );
                    break;
                }
                Some(event) = activated_stream.next() => {
                    let shortcut_id = event.shortcut_id().to_string();
                    let timestamp_ms = event.timestamp().as_millis();
                    let state = app_handle_for_task.state::<AppState>();
                    let was_recording = *state.is_recording.lock().unwrap();
                    crate::log_info!(
                        "🎤 Portal Activated: id='{}', ts={}ms, shortcut_pressed={}, is_recording={} ",
                        shortcut_id,
                        timestamp_ms,
                        shortcut_pressed,
                        was_recording
                    );

                    if shortcut_id == RECORD_SHORTCUT_ID && !shortcut_pressed {
                        shortcut_pressed = true;
                        crate::log_info!("🎤 Portal: Hotkey Pressed -> invoking start_recording");
                        let _ = crate::start_recording(state, app_handle_for_task.clone()).await;
                    } else {
                        crate::log_info!(
                            "🎤 Portal Activated ignored: id='{}', shortcut_pressed={} ",
                            shortcut_id,
                            shortcut_pressed
                        );
                    }
                }
                Some(event) = deactivated_stream.next() => {
                    let shortcut_id = event.shortcut_id().to_string();
                    let timestamp_ms = event.timestamp().as_millis();
                    let state = app_handle_for_task.state::<AppState>();
                    let was_recording = *state.is_recording.lock().unwrap();
                    crate::log_info!(
                        "⏹️  Portal Deactivated: id='{}', ts={}ms, shortcut_pressed={}, is_recording={}",
                        shortcut_id,
                        timestamp_ms,
                        shortcut_pressed,
                        was_recording
                    );

                    if shortcut_id == RECORD_SHORTCUT_ID && shortcut_pressed {
                        shortcut_pressed = false;
                        crate::log_info!("⏹️  Portal: Hotkey Released -> invoking stop_recording");
                        let _ = crate::stop_recording(state).await;
                    } else {
                        crate::log_info!(
                            "⏹️  Portal Deactivated ignored: id='{}', shortcut_pressed={}",
                            shortcut_id,
                            shortcut_pressed
                        );
                    }
                }
            }
        }

        if let Err(error) = session.close().await {
            crate::log_warn!("Failed to close global shortcut session cleanly: {}", error);
        }
    });

    Ok(())
}

fn trigger_description_matches_request(description: &str, normalized_request: &str) -> bool {
    let mut modifiers: Vec<&str> = Vec::new();
    if description.contains("<Control>") {
        modifiers.push("CTRL");
    }
    if description.contains("<Shift>") {
        modifiers.push("SHIFT");
    }
    if description.contains("<Alt>") {
        modifiers.push("ALT");
    }
    if description.contains("<Super>") {
        modifiers.push("SUPER");
    }
    modifiers.sort_unstable();

    let key = description
        .split('>')
        .last()
        .map(str::trim)
        .unwrap_or_default();

    let description_normalized = if modifiers.is_empty() {
        key.to_string()
    } else {
        format!("{}+{}", modifiers.join("+"), key)
    };

    description_normalized.eq_ignore_ascii_case(normalized_request)
}

fn trigger_description_to_hotkey(description: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if description.contains("<Control>") {
        parts.push("ctrl".to_string());
    }
    if description.contains("<Shift>") {
        parts.push("shift".to_string());
    }
    if description.contains("<Alt>") {
        parts.push("alt".to_string());
    }
    if description.contains("<Super>") {
        parts.push("super".to_string());
    }

    let key = description
        .split('>')
        .last()
        .map(str::trim)
        .filter(|segment| !segment.is_empty())?
        .to_lowercase();

    parts.push(key);
    Some(parts.join("+"))
}

#[cfg(test)]
mod tests {
    use super::normalize_wayland_trigger;

    #[test]
    fn normalize_space_shortcut() {
        assert_eq!(
            normalize_wayland_trigger("ctrl+shift+space"),
            "CTRL+SHIFT+space"
        );
    }

    #[test]
    fn normalize_enter_shortcut() {
        assert_eq!(normalize_wayland_trigger("ctrl+enter"), "CTRL+Return");
    }

    #[test]
    fn normalize_function_shortcut() {
        assert_eq!(normalize_wayland_trigger("f8"), "F8");
        assert_eq!(normalize_wayland_trigger("ctrl+f8"), "CTRL+F8");
    }
}
