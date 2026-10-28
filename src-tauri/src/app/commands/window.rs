use crate::app::tray::{self, WindowParking};

#[tauri::command]
pub async fn minimize_to_tray_or_taskbar(
    app_handle: tauri::AppHandle,
) -> Result<WindowParking, String> {
    tray::park_main_window(&app_handle).await
}

/// Title-bar close button. Returns where the window was parked, or `None`
/// when the `close_to_tray` setting is off and the application is quitting.
#[tauri::command]
pub async fn close_main_window(
    app_handle: tauri::AppHandle,
) -> Result<Option<WindowParking>, String> {
    tray::close_main_window(&app_handle).await
}
