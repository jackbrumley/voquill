pub mod input;
pub mod overlay;
pub mod permissions;
pub mod shortcuts;

use async_trait::async_trait;
use std::sync::Arc;
use tauri::WebviewWindow;

use crate::platform::traits::{
    DisplayBackend, GlobalShortcutEngine, InputSimulation, PermissionManager, WindowManagement,
};

pub struct WindowsBackend;

impl WindowsBackend {
    pub fn new() -> Self {
        Self
    }
}

pub fn initialize() -> Arc<dyn DisplayBackend> {
    Arc::new(WindowsBackend::new())
}

#[async_trait]
impl InputSimulation for WindowsBackend {
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
impl GlobalShortcutEngine for WindowsBackend {
    async fn start_engine(&self, app_handle: tauri::AppHandle, _force: bool) -> Result<(), String> {
        shortcuts::start_windows_hotkey_engine(app_handle).await
    }
}

#[async_trait]
impl PermissionManager for WindowsBackend {
    async fn request_permissions(&self, _app_handle: tauri::AppHandle) -> Result<(), String> {
        Ok(())
    }

    async fn check_permissions(
        &self,
        _config: &crate::config::Config,
    ) -> crate::platform::permissions::LinuxPermissions {
        permissions::check_windows_permissions().await
    }
}

#[async_trait]
impl WindowManagement for WindowsBackend {
    fn apply_overlay_hints(&self, window: &WebviewWindow) {
        overlay::apply_overlay_hints(window);
    }

    fn position_overlay_window(
        &self,
        window: &WebviewWindow,
        pixels_from_bottom: i32,
    ) -> Result<(), String> {
        overlay::position_overlay_window(window, pixels_from_bottom)
    }
}
