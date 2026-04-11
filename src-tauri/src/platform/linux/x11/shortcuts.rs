use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub async fn start_x11_hotkey_engine(app_handle: tauri::AppHandle) -> Result<(), String> {
    let hotkey_str = {
        let state = app_handle.state::<crate::AppState>();
        let config = state.config.lock().unwrap();
        config.hotkey.clone()
    };

    if hotkey_str.is_empty() {
        return Err("No hotkey configured for X11.".to_string());
    }

    crate::log_info!("🔄 Re-registering X11 hotkey: {}", hotkey_str);
    app_handle
        .global_shortcut()
        .unregister_all()
        .map_err(|error| format!("Failed to clear existing X11 hotkeys: {error}"))?;

    let shortcut = crate::hotkey::parse_hotkey_string(&hotkey_str)
        .map_err(|error| format!("Failed to parse hotkey string: {error}"))?;

    app_handle
        .global_shortcut()
        .register(shortcut)
        .map_err(|error| format!("Failed to register global hotkey on X11: {error}"))?;

    crate::log_info!("✅ X11 global hotkey registered: {}", hotkey_str);

    Ok(())
}
