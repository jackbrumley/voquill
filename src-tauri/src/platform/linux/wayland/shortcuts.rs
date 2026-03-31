use crate::AppState;
use tauri::{Manager, Emitter};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;



pub fn normalize_wayland_trigger(hotkey: &str) -> String {
    // Input example: "ctrl+shift+space"
    // Portal expects: "CTRL+SHIFT+space"
    let mut parts: Vec<String> = hotkey.split('+').map(|s| s.to_string()).collect();
    if parts.len() < 2 { return hotkey.to_string(); }
    
    let mut key = parts.pop().unwrap();
    // Portal keysyms are usually capitalized standard names like "Space", "Return", "A"
    if key.to_lowercase() == "space" { key = "space".to_string(); }
    else if key.to_lowercase() == "enter" || key.to_lowercase() == "return" { key = "Return".to_string(); }
    
    let mut modifiers = parts;
    for m in &mut modifiers {
        *m = m.to_uppercase();
    }
    
    modifiers.join("+") + "+" + &key
}

pub fn denormalize_wayland_trigger(trigger: &str) -> String {
    trigger.to_lowercase()
}

pub async fn start_linux_portal_hotkey_engine(app_handle: tauri::AppHandle, force: bool) -> Result<(), String> {
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

    let (shortcuts_token, hotkey_str) = {
        let config = state.config.lock().unwrap();
        (config.shortcuts_token.clone(), config.hotkey.clone())
    };

    if shortcuts_token.is_none() && !force {
        return Err("No shortcuts token found. Setup is required.".to_string());
    }

    // Implicit matching via Desktop file

    let proxy = GlobalShortcuts::new().await
        .map_err(|e| format!("Failed to connect to Portal: {}", e))?;

    let session = proxy.create_session().await
        .map_err(|e| format!("Failed to create portal session: {}", e))?;

    let normalized_trigger = normalize_wayland_trigger(&hotkey_str);
    crate::log_info!("🔑 Attempting to bind trigger: '{}' (normalized: '{}')", hotkey_str, normalized_trigger);

    let shortcut = NewShortcut::new("record", "Dictation Hotkey")
        .preferred_trigger(Some(normalized_trigger.as_str()));

    let bind_result = proxy.bind_shortcuts(&session, &[shortcut], None).await
        .map_err(|e| format!("Failed to call portal BindShortcuts: {}", e))?;

    let bound = bind_result.response()
        .map_err(|e| format!("Portal rejected shortcut: {}", e))?;

    if bound.shortcuts().is_empty() {
        return Err("OS rejected the shortcut request. The hotkey may be in use by another application or the system.".to_string());
    }
    
    let mut actual_trigger = String::new();
    for s in bound.shortcuts() {
        crate::log_info!("✅ Wayland Global Shortcuts bound: ID='{}', Trigger='{}'", s.id(), s.trigger_description());
        if s.id() == "record" {
            actual_trigger = s.trigger_description().to_string();
        }
    }

    {
        let mut config = state.config.lock().unwrap();
        config.shortcuts_token = Some("granted".to_string());
        if !actual_trigger.is_empty() {
            config.hotkey = denormalize_wayland_trigger(&actual_trigger);
        }
        let _ = crate::config::save_config(&config);
    }
    let _ = app_handle.emit("config-updated", ());

    let mut activated_stream = proxy.receive_activated().await
        .map_err(|e| format!("Failed to listen for shortcut activation: {}", e))?;

    let mut deactivated_stream = proxy.receive_deactivated().await
        .map_err(|e| format!("Failed to listen for shortcut deactivation: {}", e))?;

    let h_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        crate::log_info!("👂 Listening for shortcut events...");
        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    crate::log_info!("🛑 Hotkey engine cancelled.");
                    break;
                }
                Some(event) = activated_stream.next() => {
                    if event.shortcut_id() == "record" {
                        crate::log_info!("🎤 Portal: Hotkey Pressed");
                        let state = h_handle.state::<AppState>();
                        let _ = crate::start_recording(state, h_handle.clone()).await;
                    }
                }
                Some(event) = deactivated_stream.next() => {
                    if event.shortcut_id() == "record" {
                        crate::log_info!("⏹️  Portal: Hotkey Released");
                        let state = h_handle.state::<AppState>();
                        let _ = crate::stop_recording(state).await;
                    }
                }
            }
        }
    });

    Ok(())
}
