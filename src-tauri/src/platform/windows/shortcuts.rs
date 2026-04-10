use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub async fn start_windows_hotkey_engine(app_handle: tauri::AppHandle) -> Result<(), String> {
    let hotkey_string = {
        let state = app_handle.state::<crate::AppState>();
        let config = state.config.lock().unwrap();
        config.hotkey.clone()
    };

    crate::log_info!("Re-registering hotkey: {}", hotkey_string);
    let _ = app_handle.global_shortcut().unregister_all();

    match crate::hotkey::parse_hotkey_string(&hotkey_string) {
        Ok(shortcut) => {
            if let Err(e) = app_handle.global_shortcut().register(shortcut) {
                crate::log_info!("❌ Failed to register global hotkey: {}", e);
                return Err(format!("Failed to register global hotkey: {e}"));
            } else {
                crate::log_info!("✅ Global hotkey registered: {}", hotkey_string);
            }
        }
        Err(e) => {
            crate::log_info!("❌ Failed to parse hotkey string: {}", e);
            return Err(format!("Failed to parse hotkey string: {e}"));
        }
    }

    Ok(())
}
