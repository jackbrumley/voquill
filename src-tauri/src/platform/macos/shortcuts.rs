use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub async fn start_macos_hotkey_engine(app_handle: tauri::AppHandle) -> Result<(), String> {
    let hotkey = {
        let state = app_handle.state::<crate::AppState>();
        let hotkey = state.config.lock().unwrap().hotkey.clone();
        hotkey
    };

    if hotkey.is_empty() {
        return Err("No hotkey configured for macOS.".to_string());
    }

    app_handle
        .global_shortcut()
        .unregister_all()
        .map_err(|error| format!("Failed to clear existing macOS hotkeys: {error}"))?;
    let shortcut = crate::hotkey::parse_hotkey_string(&hotkey)
        .map_err(|error| format!("Failed to parse hotkey string: {error}"))?;
    app_handle
        .global_shortcut()
        .register(shortcut)
        .map_err(|error| format!("Failed to register global hotkey on macOS: {error}"))?;

    crate::log_info!("macOS global hotkey registered: {}", hotkey);
    Ok(())
}
