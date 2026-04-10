pub mod env;
pub mod input;
pub mod overlay;
pub mod permissions;
pub mod portal;
pub mod shortcuts;

use async_trait::async_trait;
use tauri::{Manager, WebviewWindow};

use crate::platform::traits::{
    GlobalShortcutEngine, InputSimulation, PermissionManager, WindowManagement,
};

pub struct WaylandBackend;

impl WaylandBackend {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl InputSimulation for WaylandBackend {
    async fn type_text_hardware(
        &self,
        app_handle: &tauri::AppHandle,
        text: &str,
        typing_speed_interval: f64,
        key_press_duration_ms: u64,
    ) -> Result<(), String> {
        input::type_text_hardware(
            app_handle,
            text,
            typing_speed_interval,
            key_press_duration_ms,
        )
        .await
    }
}

#[async_trait]
impl GlobalShortcutEngine for WaylandBackend {
    async fn start_engine(&self, app_handle: tauri::AppHandle, force: bool) -> Result<(), String> {
        shortcuts::start_linux_portal_hotkey_engine(app_handle, force).await
    }
}

#[async_trait]
impl PermissionManager for WaylandBackend {
    async fn request_permissions(&self, app_handle: tauri::AppHandle) -> Result<(), String> {
        permissions::request_linux_permissions(app_handle).await
    }

    async fn check_permissions(
        &self,
        config: &crate::config::Config,
    ) -> crate::platform::permissions::LinuxPermissions {
        permissions::check_linux_permissions(config).await
    }
}

#[async_trait]
impl WindowManagement for WaylandBackend {
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
