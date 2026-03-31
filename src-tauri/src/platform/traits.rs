use async_trait::async_trait;
use tauri::{AppHandle, WebviewWindow};
use crate::config::Config;
use crate::platform::permissions::LinuxPermissions;

#[async_trait]
pub trait InputSimulation: Send + Sync {
    fn type_text_hardware(&self, text: &str, typing_speed_interval: f64, key_press_duration_ms: u64, virtual_keyboard: std::sync::Arc<std::sync::Mutex<Option<crate::VirtualKeyboardHandle>>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

#[async_trait]
pub trait GlobalShortcutEngine: Send + Sync {
    async fn start_engine(&self, app_handle: AppHandle, force: bool) -> Result<(), String>;
}

#[async_trait]
pub trait PermissionManager: Send + Sync {
    async fn request_permissions(&self, app_handle: AppHandle) -> Result<(), String>;
    async fn check_permissions(&self, config: &Config) -> LinuxPermissions;
}

#[async_trait]
pub trait WindowManagement: Send + Sync {
    fn apply_overlay_hints(&self, window: &WebviewWindow);
    fn position_overlay_window(&self, window: &WebviewWindow, pixels_from_bottom: i32) -> Result<(), String>;
}

pub trait DisplayBackend:
    InputSimulation + GlobalShortcutEngine + PermissionManager + WindowManagement + Send + Sync
{
}

impl<T> DisplayBackend for T where
    T: InputSimulation + GlobalShortcutEngine + PermissionManager + WindowManagement + Send + Sync
{
}

