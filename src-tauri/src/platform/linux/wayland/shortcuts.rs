use crate::AppState;
use tauri::{Manager, Emitter};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures_util::StreamExt;

pub fn normalize_wayland_trigger(hotkey: &str) -> String {
    // Input example: "ctrl+shift+space"
    // Portal expects: "CTRL+SHIFT+space"
    let mut parts: Vec<String> = hotkey.split('+').map(|s| s.to_string()).collect();
    if parts.len() < 2 { return hotkey.to_string(); }
    
    let key = parts.pop().unwrap();
    let mut modifiers = parts;
    for m in &mut modifiers {
        *m = m.to_uppercase();
    }
    
    modifiers.join("+") + "+" + &key
}

pub fn denormalize_wayland_trigger(trigger: &str) -> String {
    trigger.to_lowercase()
}

pub async fn start_linux_portal_hotkey_engine(app_handle: tauri::AppHandle, force: bool, manual_prompt: bool) {
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
        crate::log_info!("⚠️ No shortcuts token found. Skipping hotkey registration.");
        return;
    }

    let proxy = match GlobalShortcuts::new().await {
        Ok(p) => p,
        Err(e) => {
            crate::log_info!("❌ Failed to connect to Global Shortcuts Portal: {}", e);
            return;
        }
    };

    let session = match proxy.create_session().await {
        Ok(s) => s,
        Err(e) => {
            crate::log_info!("❌ Failed to create shortcuts session: {}", e);
            return;
        }
    };

    let normalized_trigger = normalize_wayland_trigger(&hotkey_str);
    crate::log_info!("🔑 Attempting to bind trigger: '{}' (normalized: '{}')", hotkey_str, normalized_trigger);

    let shortcut = NewShortcut::new("record", "Dictation Hotkey");
    let shortcut = if !manual_prompt {
        shortcut.preferred_trigger(Some(normalized_trigger.as_str()))
    } else {
        shortcut
    };

    let bind_result = match proxy.bind_shortcuts(&session, &[shortcut], None).await {
        Ok(r) => r,
        Err(e) => {
            crate::log_info!("❌ Failed to bind shortcuts: {}", e);
            if manual_prompt {
                if let Err(e2) = proxy.configure_shortcuts(&session, None, None).await {
                    crate::log_info!("❌ Portal does not support manual configuration: {}", e2);
                }
            }
            return;
        }
    };

    let bound = match bind_result.response() {
        Ok(shortcuts) => shortcuts,
        Err(e) => {
            crate::log_info!("❌ Failed to get bind response: {}", e);
            return;
        }
    };

    if bound.shortcuts().is_empty() {
        crate::log_info!("⚠️ OS rejected the shortcut request. No shortcuts were bound.");
        return;
    }
    
    for s in bound.shortcuts() {
        crate::log_info!("✅ Wayland Global Shortcuts bound: ID='{}', Trigger='{}'", s.id(), s.trigger_description());
    }

    let mut activated_stream = match proxy.receive_activated().await {
        Ok(s) => s,
        Err(e) => {
            crate::log_info!("❌ Failed to listen for shortcut activation: {}", e);
            return;
        }
    };

    let mut deactivated_stream = match proxy.receive_deactivated().await {
        Ok(s) => s,
        Err(e) => {
            crate::log_info!("❌ Failed to listen for shortcut deactivation: {}", e);
            return;
        }
    };

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
}
