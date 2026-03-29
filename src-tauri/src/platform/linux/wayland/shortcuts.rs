use crate::AppState;
use tauri::{Manager, Emitter};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;

pub fn denormalize_wayland_trigger(trigger: &str) -> String {
    // Converts PORTAL+FORMAT (CTRL+SHIFT+space) to user-friendly format (ctrl+shift+space)
    let parts: Vec<&str> = trigger.split('+').collect();
    let mut normalized_parts: Vec<String> = Vec::new();
    
    for part in parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "logo" => normalized_parts.push("super".to_string()),
            _ => normalized_parts.push(lower),
        }
    }
    
    normalized_parts.join("+")
}

pub fn normalize_wayland_trigger(hotkey: &str) -> String {
    // Freedesktop Shortcuts Specification format: MOD+MOD+KeySymName
    // https://specifications.freedesktop.org/shortcuts-spec/latest/
    // Modifiers: CTRL, ALT, SHIFT, LOGO (for Super/Meta)
    // Keys: lowercase xkbcommon keysym names (e.g., space, h, Return)
    
    let parts: Vec<&str> = hotkey.split('+').collect();
    let mut modifiers: Vec<&str> = Vec::new();
    let mut primary = String::new();
    
    for part in parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "ctrl" | "control" => modifiers.push("CTRL"),
            "alt" => modifiers.push("ALT"),
            "shift" => modifiers.push("SHIFT"),
            "super" | "meta" => modifiers.push("LOGO"),
            _ => {
                // Map to xkbcommon keysym names (lowercase, special handling)
                primary = match lower.as_str() {
                    "space" => "space".to_string(),
                    "enter" | "return" => "Return".to_string(),
                    "backspace" => "BackSpace".to_string(),
                    "tab" => "Tab".to_string(),
                    "escape" | "esc" => "Escape".to_string(),
                    _ => lower,
                };
            }
        }
    }
    
    // Build the final trigger: MOD+MOD+Key
    let mut result = modifiers.join("+");
    if !result.is_empty() && !primary.is_empty() {
        result.push('+');
    }
    result.push_str(&primary);
    
    result
}

pub async fn start_linux_portal_hotkey_engine(app_handle: tauri::AppHandle, force: bool, manual_prompt: bool) {
    // Cancel any existing engine before starting a new one
    let state = app_handle.state::<AppState>();
    
    // First, cancel the old engine and WAIT for it to finish
    let has_previous = {
        let mut cancel_lock = state.hotkey_engine_cancel.lock().unwrap();
        if let Some(sender) = cancel_lock.take() {
            log_info!("🔄 Cancelling previous hotkey engine...");
            let _ = sender.send(());
            true
        } else {
            false
        }
    };
    
    if has_previous {
        // Give the old engine more time to shut down to avoid race conditions
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
    }
    
    // Create a cancellation channel for this new engine
    let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let mut cancel_lock = state.hotkey_engine_cancel.lock().unwrap();
        *cancel_lock = Some(cancel_tx);
    }

    log_info!("🚀 Wayland Global Shortcuts Engine starting...");

    let (shortcuts_token, hotkey_str) = {
        let config = state.config.lock().unwrap();
        (config.shortcuts_token.clone(), config.hotkey.clone())
    };

    if shortcuts_token.is_none() && !force {
        log_info!("⚠️ No shortcuts token found and force=false. Skipping hotkey registration until setup is run.");
        return;
    }

    let proxy = match GlobalShortcuts::new().await {
        Ok(p) => p,
        Err(e) => {
            log_info!("❌ Failed to connect to Global Shortcuts Portal: {}", e);
            return;
        }
    };

    // Restore session
    let session = match proxy.create_session().await {
        Ok(s) => s,
        Err(e) => {
            log_info!("❌ Failed to create shortcuts session: {}", e);
            return;
        }
    };

    // Normalize the hotkey to XDG/Portal format
    let normalized_trigger = normalize_wayland_trigger(&hotkey_str);
    log_info!("🔑 Attempting to bind trigger: '{}'", normalized_trigger);

    // Bind shortcuts
    let shortcut = if manual_prompt {
        log_info!("🧭 Manual shortcut selection requested; prompting portal UI");
        NewShortcut::new("record", "Dictation Hotkey")
    } else {
        NewShortcut::new("record", "Dictation Hotkey")
            .preferred_trigger(Some(normalized_trigger.as_str()))
    };

    let bind_result = match proxy.bind_shortcuts(&session, &[shortcut], None).await {
        Ok(r) => r,
        Err(e) => {
            log_info!("❌ Failed to bind shortcuts: {}", e);
            return;
        }
    };

    // Verify what the OS actually bound
    let bound = match bind_result.response() {
        Ok(shortcuts) => shortcuts,
        Err(e) => {
            log_info!("❌ Failed to get bind response: {}", e);
            return;
        }
    };

    if bound.shortcuts().is_empty() {
        log_info!("⚠️ OS rejected the shortcut request. No shortcuts were bound.");
        log_info!("💡 This may mean the key combination is already in use by the system.");
        return;
    }
    
    let mut actual_trigger = String::new();
    for s in bound.shortcuts() {
        log_info!("✅ Wayland Global Shortcuts bound: ID='{}', Description='{}', Trigger='{}'", 
            s.id(), s.description(), s.trigger_description());
        if s.id() == "record" {
            actual_trigger = s.trigger_description().to_string();
        }
    }

    if actual_trigger.is_empty() {
        log_info!("⚠️ OS accepted the registration but didn't assign a trigger.");
        log_info!("💡 KDE Plasma: You may need to manually assign the shortcut in System Settings → Shortcuts");
        log_info!("💡 Or check: journalctl --user -u xdg-desktop-portal-kde -n 50");
    } else {
        log_info!("🎉 Trigger successfully bound: '{}'", actual_trigger);
        
        let friendly_hotkey = denormalize_wayland_trigger(&actual_trigger);
        
        // Save the token and the actual bound hotkey
        {
            let mut config = state.config.lock().unwrap();
            config.shortcuts_token = Some("granted".to_string());
            config.hotkey = friendly_hotkey;
            let _ = crate::config::save_config(&config);
        }
        
        // Notify the frontend to refresh its config
        let _ = app_handle.emit("config-updated", ());
    }

    let mut activated_stream = match proxy.receive_activated().await {
        Ok(s) => s,
        Err(e) => {
            log_info!("❌ Failed to listen for shortcut activation: {}", e);
            return;
        }
    };

    let mut deactivated_stream = match proxy.receive_deactivated().await {
        Ok(s) => s,
        Err(e) => {
            log_info!("❌ Failed to listen for shortcut deactivation: {}", e);
            return;
        }
    };

    let h_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        log_info!("👂 Listening for shortcut events...");
        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    log_info!("🛑 Hotkey engine cancelled. Shutting down...");
                    break;
                }
                Some(event) = activated_stream.next() => {
                    log_info!("🔔 Portal event received: Activated shortcut_id='{}'", event.shortcut_id());
                    if event.shortcut_id() == "record" {
                        log_info!("🎤 Portal: Hotkey Pressed");
                        let state = h_handle.state::<AppState>();
                        let _ = crate::start_recording(state, h_handle.clone()).await;
                        if let Some(w) = h_handle.get_webview_window("main") {
                            let _ = w.emit("hotkey-pressed", ());
                        }
                    } else {
                        log_info!("⚠️ Unknown shortcut activated: {}", event.shortcut_id());
                    }
                }
                Some(event) = deactivated_stream.next() => {
                    log_info!("🔔 Portal event received: Deactivated shortcut_id='{}'", event.shortcut_id());
                    if event.shortcut_id() == "record" {
                        log_info!("⏹️  Portal: Hotkey Released");
                        let state = h_handle.state::<AppState>();
                        let _ = crate::stop_recording(state).await;
                        if let Some(w) = h_handle.get_webview_window("main") {
                            let _ = w.emit("hotkey-released", ());
                        }
                    } else {
                        log_info!("⚠️ Unknown shortcut deactivated: {}", event.shortcut_id());
                    }
                }
            }
        }
        log_info!("✅ Hotkey engine stopped cleanly.");
    });
}
