pub mod input;
pub mod overlay;
pub mod permissions;
pub mod shortcuts;

use async_trait::async_trait;
use std::sync::Arc;
use tauri::{Manager, WebviewWindow};

use crate::platform::traits::{
    DisplayBackend, GlobalShortcutEngine, InputSimulation, PermissionManager, WindowManagement,
};

#[derive(Default)]
pub struct MacosBackend;

pub fn initialize() -> Arc<dyn DisplayBackend> {
    Arc::new(MacosBackend)
}

#[async_trait]
impl InputSimulation for MacosBackend {
    fn supported_paste_shortcuts(&self) -> &'static [crate::config::PasteShortcut] {
        &[crate::config::PasteShortcut::CommandV]
    }

    async fn type_text_hardware(
        &self,
        app_handle: &tauri::AppHandle,
        text: &str,
        typing_interval_ms: u64,
        key_press_duration_ms: u64,
    ) -> Result<(), String> {
        let session_state = app_handle.state::<crate::AppState>().session_state.clone();
        let text = text.to_string();
        tokio::task::spawn_blocking(move || {
            input::type_text_hardware(
                &text,
                typing_interval_ms,
                key_press_duration_ms,
                &session_state,
            )
        })
        .await
        .map_err(|error| format!("macOS typing task failed: {error}"))?
    }

    async fn send_paste_shortcut(
        &self,
        _app_handle: &tauri::AppHandle,
        shortcut: crate::config::PasteShortcut,
    ) -> Result<(), String> {
        input::send_paste_shortcut(shortcut)
    }

    async fn send_key_combination(
        &self,
        _app_handle: &tauri::AppHandle,
        combination: &str,
        hold_duration_ms: u64,
    ) -> Result<(), String> {
        input::send_key_combination(combination, hold_duration_ms)
    }

    async fn send_key_down(&self, _app_handle: &tauri::AppHandle, key: &str) -> Result<(), String> {
        input::send_key_down(key)
    }

    async fn send_key_up(&self, _app_handle: &tauri::AppHandle, key: &str) -> Result<(), String> {
        input::send_key_up(key)
    }
}

#[async_trait]
impl GlobalShortcutEngine for MacosBackend {
    fn supports_fn_modifier(&self) -> bool {
        true
    }

    async fn start_engine(&self, app_handle: tauri::AppHandle, _force: bool) -> Result<(), String> {
        shortcuts::start_macos_hotkey_engine(app_handle).await
    }
}

#[async_trait]
impl PermissionManager for MacosBackend {
    /// CoreAudio shows the microphone prompt when an input stream opens, so
    /// briefly holding the microphone is the request. The engine stays open
    /// afterwards only if the settings keep it open.
    async fn request_audio_permission(&self, app_handle: tauri::AppHandle) -> Result<(), String> {
        let _permission_hold = crate::app::capture::hold(
            &app_handle,
            crate::app::capture::CapturePurpose::PermissionRequest,
        )?;
        Ok(())
    }

    async fn request_input_permission(&self, app_handle: tauri::AppHandle) -> Result<(), String> {
        let hotkey = app_handle
            .state::<crate::AppState>()
            .config
            .lock()
            .unwrap()
            .hotkey
            .clone();
        permissions::request_input_permissions(&hotkey)
    }

    async fn check_permissions(
        &self,
        config: &crate::config::Config,
    ) -> crate::platform::permissions::PlatformPermissions {
        permissions::check_macos_permissions(config)
    }
}

impl WindowManagement for MacosBackend {
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
