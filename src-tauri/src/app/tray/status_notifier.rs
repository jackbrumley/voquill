//! Linux tray, published as a StatusNotifierItem over DBus.
//!
//! Tauri's Linux tray (libappindicator) reports no clicks, so any click opens
//! the menu. Speaking StatusNotifierItem directly lets the desktop's tray host
//! (KDE Plasma, Cinnamon, GNOME with the AppIndicator extension) send
//! `Activate` on left click, matching Windows and macOS.

use super::{handle_menu_item, show_main_window, TrayMenuItem};
use crate::platform::linux::wayland::env::APP_ID;
use ksni::menu::StandardItem;
use ksni::{Icon, MenuItem, Tray, TrayMethods};

/// The watcher that tray hosts run and StatusNotifierItems register with.
const WATCHER_BUS_NAME: &str = "org.kde.StatusNotifierWatcher";

struct VoquillTray {
    app_handle: tauri::AppHandle,
    icon: Icon,
}

impl Tray for VoquillTray {
    fn id(&self) -> String {
        APP_ID.to_string()
    }

    fn title(&self) -> String {
        "Voquill".to_string()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![self.icon.clone()]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        show_main_window(&self.app_handle);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        TrayMenuItem::ALL
            .into_iter()
            .map(|item| {
                StandardItem {
                    label: item.label().to_string(),
                    activate: Box::new(move |tray: &mut Self| {
                        handle_menu_item(&tray.app_handle, item)
                    }),
                    ..Default::default()
                }
                .into()
            })
            .collect()
    }
}

/// Publishes the tray icon. A missing watcher is not an error: the item
/// registers as soon as a tray host appears (e.g. the GNOME extension is
/// enabled later), and `is_watcher_available` drives the taskbar fallback
/// meanwhile.
pub fn spawn(app_handle: &tauri::AppHandle, icon: &tauri::image::Image<'_>) {
    let tray = VoquillTray {
        app_handle: app_handle.clone(),
        icon: Icon {
            width: icon.width() as i32,
            height: icon.height() as i32,
            data: rgba_to_argb(icon.rgba()),
        },
    };
    tauri::async_runtime::spawn(async move {
        match tray.assume_sni_available(true).spawn().await {
            Ok(_handle) => crate::log_info!("Tray icon published as a StatusNotifierItem"),
            Err(error) => crate::log_warn!("Failed to publish tray icon: {}", error),
        }
    });
}

/// StatusNotifierItem pixmaps are ARGB32 in network byte order; Tauri icons
/// are RGBA.
fn rgba_to_argb(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|pixel| [pixel[3], pixel[0], pixel[1], pixel[2]])
        .collect()
}

pub async fn is_watcher_available() -> bool {
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

    let watcher = match BusName::try_from(WATCHER_BUS_NAME) {
        Ok(name) => name,
        Err(error) => {
            crate::log_warn!("Invalid tray watcher bus name: {}", error);
            return false;
        }
    };

    match proxy.name_has_owner(watcher).await {
        Ok(available) => available,
        Err(error) => {
            crate::log_warn!("Failed to check tray watcher: {}", error);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::rgba_to_argb;

    #[test]
    fn converts_rgba_pixels_to_argb() {
        let rgba = [10, 20, 30, 255, 1, 2, 3, 128];
        assert_eq!(rgba_to_argb(&rgba), vec![255, 10, 20, 30, 128, 1, 2, 3]);
    }
}
