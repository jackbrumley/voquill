//! Owns the tray icon, tray availability, and every way the main window is
//! shown, parked or quit. The tray backends differ per platform (Tauri's tray
//! on Windows/macOS, a StatusNotifierItem on Linux) but share one menu, one
//! click behavior and one set of actions defined here:
//! - left click opens the main window;
//! - right click shows the menu (Open Voquill / Quit).

#[cfg(target_os = "linux")]
mod status_notifier;
#[cfg(not(target_os = "linux"))]
mod system_tray;

use crate::app::state::AppState;
use serde::Serialize;
use tauri::Manager;

/// Where the main window went when it was taken off screen without quitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowParking {
    Tray,
    Taskbar,
}

/// The tray menu, in display order. Both tray backends build their menu from
/// this list and route selections through `handle_menu_item`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrayMenuItem {
    Open,
    Quit,
}

impl TrayMenuItem {
    const ALL: [Self; 2] = [Self::Open, Self::Quit];

    fn label(self) -> &'static str {
        match self {
            Self::Open => "Open Voquill",
            Self::Quit => "Quit",
        }
    }
}

fn handle_menu_item(app_handle: &tauri::AppHandle, item: TrayMenuItem) {
    match item {
        TrayMenuItem::Open => show_main_window(app_handle),
        TrayMenuItem::Quit => quit(app_handle),
    }
}

/// Creates the tray icon for this platform.
pub fn build_tray(app: &tauri::App<tauri::Wry>) -> Result<(), Box<dyn std::error::Error>> {
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("Voquill has no bundled icon to show in the system tray")?;

    #[cfg(target_os = "linux")]
    status_notifier::spawn(app.handle(), &icon);

    #[cfg(not(target_os = "linux"))]
    system_tray::build(app, icon)?;

    Ok(())
}

/// Routes every close request (title-bar button, Alt+F4, taskbar "Close
/// window") through `close_main_window` so the `close_to_tray` setting is
/// honored no matter how the window was closed.
pub fn intercept_main_window_close(app: &tauri::App<tauri::Wry>) {
    let Some(window) = app.get_webview_window("main") else {
        crate::log_warn!("Main window not found; close-to-tray handling not installed");
        return;
    };
    let app_handle = app.handle().clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = close_main_window(&app_handle).await {
                    crate::log_warn!("Failed to close main window: {}", error);
                }
            });
        }
    });
}

/// Whether a system tray host is present to receive the tray icon. Windows
/// and macOS always provide one; Linux depends on a StatusNotifier watcher
/// (absent on stock GNOME without the AppIndicator extension).
pub async fn is_tray_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        status_notifier::is_watcher_available().await
    }

    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

/// Takes the main window off screen without quitting: hidden to the tray
/// when one exists, otherwise minimized to the taskbar so it stays reachable.
pub async fn park_main_window(app_handle: &tauri::AppHandle) -> Result<WindowParking, String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    if is_tray_available().await {
        window.hide().map_err(|error| error.to_string())?;
        return Ok(WindowParking::Tray);
    }

    window.minimize().map_err(|error| error.to_string())?;
    Ok(WindowParking::Taskbar)
}

/// Closes the main window according to the `close_to_tray` setting. Returns
/// where the window was parked, or `None` when the application is quitting.
pub async fn close_main_window(
    app_handle: &tauri::AppHandle,
) -> Result<Option<WindowParking>, String> {
    let close_to_tray = app_handle
        .state::<AppState>()
        .config
        .lock()
        .unwrap()
        .close_to_tray;

    if !close_to_tray {
        quit(app_handle);
        return Ok(None);
    }

    let parking = park_main_window(app_handle).await?;
    if parking == WindowParking::Taskbar {
        crate::log_warn!(
            "Close to tray requested but no system tray is available; minimized to taskbar instead"
        );
    }
    Ok(Some(parking))
}

/// The single path for bringing Voquill forward: tray click, tray menu, a
/// second launch, and (on macOS) clicking the Dock icon all end here.
pub fn show_main_window(app_handle: &tauri::AppHandle) {
    let Some(window) = app_handle.get_webview_window("main") else {
        crate::log_warn!("Cannot show the main window: window not found");
        return;
    };
    if let Err(error) = window.unminimize() {
        crate::log_warn!("Failed to unminimize the main window: {}", error);
    }
    if let Err(error) = window.show() {
        crate::log_warn!("Failed to show the main window: {}", error);
    }
    if let Err(error) = window.set_focus() {
        crate::log_warn!("Failed to focus the main window: {}", error);
    }
}

/// A second launch (start menu, desktop file, autostart) hands its arguments
/// to this instance and exits. Bring the window forward unless that launch
/// asked to start hidden.
pub fn handle_second_launch(app_handle: &tauri::AppHandle, show_window: bool) {
    if show_window {
        crate::log_info!("Second launch detected; showing the main window");
        show_main_window(app_handle);
    } else {
        crate::log_info!("Second launch detected with --start-hidden; leaving the window as is");
    }
}

/// Exits the app; the `RunEvent::Exit` handler releases its resources.
pub fn quit(app_handle: &tauri::AppHandle) {
    app_handle.exit(0);
}

#[cfg(test)]
mod tests {
    use super::TrayMenuItem;

    #[test]
    fn menu_lists_open_before_quit() {
        assert_eq!(TrayMenuItem::ALL, [TrayMenuItem::Open, TrayMenuItem::Quit]);
    }
}
