use crate::app::state::AppState;
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager,
};

/// Where the main window went when it was taken off screen without quitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowParking {
    Tray,
    Taskbar,
}

fn create_tray_menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let open_item = MenuItem::with_id(app, "open", "Open Voquill", true, None::<&str>)?;
    Menu::with_items(app, &[&open_item, &quit_item])
}

pub fn build_tray(app: &tauri::App<tauri::Wry>) -> Result<(), tauri::Error> {
    let menu = create_tray_menu(app.handle())?;
    TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(|app_handle, event| match event.id.as_ref() {
            "quit" => quit(app_handle),
            "open" => show_main_window(app_handle),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
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
        is_status_notifier_watcher_available().await
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

pub fn show_main_window(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn quit(app_handle: &tauri::AppHandle) {
    app_handle.state::<AppState>().cleanup();
    app_handle.exit(0);
}

#[cfg(target_os = "linux")]
async fn is_status_notifier_watcher_available() -> bool {
    use zbus::names::BusName;

    let connection = match zbus::Connection::session().await {
        Ok(connection) => connection,
        Err(error) => {
            crate::log_warn!("Failed to open session DBus for tray check: {}", error);
            return false;
        }
    };

    let proxy = match zbus::fdo::DBusProxy::new(&connection).await {
        Ok(proxy) => proxy,
        Err(error) => {
            crate::log_warn!("Failed to create DBus proxy for tray check: {}", error);
            return false;
        }
    };

    let kde_watcher = match BusName::try_from("org.kde.StatusNotifierWatcher") {
        Ok(name) => name,
        Err(error) => {
            crate::log_warn!("Invalid KDE watcher bus name: {}", error);
            return false;
        }
    };
    let freedesktop_watcher = match BusName::try_from("org.freedesktop.StatusNotifierWatcher") {
        Ok(name) => name,
        Err(error) => {
            crate::log_warn!("Invalid freedesktop watcher bus name: {}", error);
            return false;
        }
    };

    match proxy.name_has_owner(kde_watcher).await {
        Ok(true) => true,
        Ok(false) => match proxy.name_has_owner(freedesktop_watcher).await {
            Ok(value) => value,
            Err(error) => {
                crate::log_warn!("Failed to check freedesktop tray watcher: {}", error);
                false
            }
        },
        Err(error) => {
            crate::log_warn!("Failed to check KDE tray watcher: {}", error);
            false
        }
    }
}
