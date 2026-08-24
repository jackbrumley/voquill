use tauri::{command, AppHandle, State};

use crate::AppState;

#[command]
pub async fn test_voice_macro_sound(state: State<'_, AppState>) -> Result<(), String> {
    let playback_device = {
        let config = state.config.lock().unwrap();
        config.playback_device.clone()
    };
    crate::voice_macro::play_macro_trigger_sound(playback_device);
    Ok(())
}

#[command]
pub async fn test_voice_macro_execution(
    app_handle: AppHandle,
    steps: Vec<crate::config::MacroStep>,
) -> Result<(), String> {
    crate::voice_macro::execute_macro_steps(&app_handle, "Test Macro", &steps).await
}
