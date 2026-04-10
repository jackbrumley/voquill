pub mod input;
pub mod overlay;
pub mod permissions;
pub mod shortcuts;

use async_trait::async_trait;
use tauri::{Manager, WebviewWindow};

use crate::platform::traits::{
    GlobalShortcutEngine, InputSimulation, PermissionManager, WindowManagement,
};

pub struct X11Backend;

impl X11Backend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl InputSimulation for X11Backend {
    async fn type_text_hardware(
        &self,
        _app_handle: &tauri::AppHandle,
        text: &str,
        typing_speed_interval: f64,
        key_press_duration_ms: u64,
    ) -> Result<(), String> {
        input::type_text_hardware(text, typing_speed_interval, key_press_duration_ms)
            .map_err(|error| error.to_string())
    }
}

#[async_trait]
impl GlobalShortcutEngine for X11Backend {
    async fn start_engine(&self, app_handle: tauri::AppHandle, _force: bool) -> Result<(), String> {
        shortcuts::start_x11_hotkey_engine(app_handle).await
    }
}

#[async_trait]
impl PermissionManager for X11Backend {
    async fn request_permissions(&self, _app_handle: tauri::AppHandle) -> Result<(), String> {
        // X11 has no implicit portals; we just say "OK"
        Ok(())
    }

    async fn check_permissions(
        &self,
        _config: &crate::config::Config,
    ) -> crate::platform::permissions::LinuxPermissions {
        // All "true" because X11 is open
        crate::platform::permissions::LinuxPermissions {
            audio: true,
            shortcuts: true,
            input_emulation: true,
            shortcuts_status: "ready".to_string(),
            shortcuts_detail: None,
        }
    }
}

#[async_trait]
impl WindowManagement for X11Backend {
    fn apply_overlay_hints(&self, window: &WebviewWindow) {
        let app_handle = window.app_handle();
        let app_state = app_handle.state::<crate::AppState>();
        let pixels_from_bottom_logical = {
            let config = app_state.config.lock().unwrap();
            config.pixels_from_bottom
        };
        overlay::apply_linux_unfocusable_hints(window, pixels_from_bottom_logical);
    }

    fn position_overlay_window(
        &self,
        window: &WebviewWindow,
        pixels_from_bottom: i32,
    ) -> Result<(), String> {
        overlay::position_overlay_window(window, pixels_from_bottom)
    }
}
