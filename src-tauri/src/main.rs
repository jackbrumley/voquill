// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Logging macro with timestamps
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        println!("[{}] {}", ::chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        eprintln!("[{}] WARNING: {}", ::chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), format!($($arg)*))
    };
}

use std::sync::{Arc, Mutex, OnceLock};
use tauri::{
    Manager, WebviewWindow, Emitter, menu::{Menu, MenuItem}, tray::{TrayIconBuilder, TrayIconEvent}, AppHandle,
};

// Global app handle for emitting events - using OnceLock for thread safety
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

mod audio;
mod config;
mod history;
mod hotkey;
mod transcription;
mod typing;
mod model_manager;
mod local_whisper;
pub mod platform;

use config::{Config, TranscriptionMode};
use hotkey::HardwareHotkey;
#[cfg(target_os = "linux")]
use platform::linux::wayland::env::{enforce_wayland, check_wayland_display};
use platform::permissions::LinuxPermissions;

#[cfg(target_os = "linux")]
// Application state
#[cfg(target_os = "linux")]
pub type VirtualKeyboardHandle = evdev::uinput::VirtualDevice;
#[cfg(not(target_os = "linux"))]
pub type VirtualKeyboardHandle = ();

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub is_recording: Arc<Mutex<bool>>,
    pub is_mic_test_active: Arc<Mutex<bool>>,
    pub is_configuring_hotkey: Arc<Mutex<bool>>,
    pub hotkey_error: Arc<Mutex<Option<String>>>,
    pub setup_status: Arc<Mutex<Option<String>>>,
    pub hardware_hotkey: Arc<Mutex<HardwareHotkey>>,
    pub cached_device: Arc<Mutex<Option<cpal::Device>>>,
    pub virtual_keyboard: Arc<Mutex<Option<VirtualKeyboardHandle>>>,
    pub playback_stream: Arc<Mutex<Option<cpal::Stream>>>,
    pub mic_test_samples: Arc<Mutex<Vec<f32>>>,
    pub audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
    #[cfg(target_os = "linux")]
    pub hotkey_engine_cancel: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    pub display_backend: Arc<dyn platform::traits::DisplayBackend>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Arc::new(Mutex::new(Config::default())),
            is_recording: Arc::new(Mutex::new(false)),
            is_mic_test_active: Arc::new(Mutex::new(false)),
            is_configuring_hotkey: Arc::new(Mutex::new(false)),
            hotkey_error: Arc::new(Mutex::new(None)),
            setup_status: Arc::new(Mutex::new(None)),
            hardware_hotkey: Arc::new(Mutex::new(HardwareHotkey::default())),
            cached_device: Arc::new(Mutex::new(None)),
            virtual_keyboard: Arc::new(Mutex::new(None)),
            playback_stream: Arc::new(Mutex::new(None)),
            mic_test_samples: Arc::new(Mutex::new(Vec::new())),
            audio_engine: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            hotkey_engine_cancel: Arc::new(Mutex::new(None)),
            display_backend: platform::initialize(),
        }
    }
}

#[tauri::command]
async fn get_wayland_portal_version() -> Result<u32, String> {
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            use ashpd::desktop::global_shortcuts::GlobalShortcuts;
            if let Ok(proxy) = GlobalShortcuts::new().await {
                // In ashpd 0.12, GlobalShortcuts implements Deref to zbus::Proxy directly,
                // but the high-level `Proxy::version()` is not accessible since `self.0` is private
                // However, we can just query the property manually
                use std::ops::Deref;
                if let Ok(version) = proxy.deref().get_property::<u32>("version").await {
                    return Ok(version);
                }
            }
            return Ok(1); // Default to 1 on Wayland if we can't fetch it
        }
    }
    Ok(0)
}

#[tauri::command]
async fn get_linux_setup_status(state: tauri::State<'_, AppState>) -> Result<LinuxPermissions, String> {
    log_info!("📡 Tauri Command: get_linux_setup_status invoked");
    let config = {
        let guard = state.config.lock().unwrap();
        guard.clone()
    };
    Ok(state.display_backend.check_permissions(&config).await)
}

#[tauri::command]
async fn request_audio_permission() -> Result<(), String> {
    log_info!("📡 Tauri Command: request_audio_permission invoked");
    #[cfg(target_os = "linux")]
    {
        use ashpd::desktop::camera::Camera;
        let camera = Camera::new().await.map_err(|e| format!("Audio Portal not available: {}. Is xdg-desktop-portal installed?", e))?;
        camera.request_access().await.map_err(|e| format!("Audio access denied: {}", e))?;
        return Ok(());
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(())
    }
}

#[tauri::command]
async fn request_input_permission(state: tauri::State<'_, AppState>) -> Result<(), String> {
    log_info!("📡 Tauri Command: request_input_permission invoked");
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WAYLAND_DISPLAY").is_ok() {
            use ashpd::desktop::remote_desktop::{RemoteDesktop, DeviceType};
            use ashpd::desktop::PersistMode;

            let remote_desktop = RemoteDesktop::new().await.map_err(|e| format!("Remote Desktop Portal not available: {}", e))?;
            let rd_session = remote_desktop.create_session().await.map_err(|e| format!("Failed to create remote desktop session: {}", e))?;
            
            let select_request = remote_desktop.select_devices(&rd_session, DeviceType::Keyboard.into(), None, PersistMode::DoNot).await.map_err(|e| format!("Failed to select devices: {}", e))?;
            select_request.response().map_err(|e| format!("Device selection cancelled: {}", e))?;
            
            let start_request = remote_desktop.start(&rd_session, None).await.map_err(|e| format!("Failed to start remote desktop session: {}", e))?;
            let selected_devices = start_request.response().map_err(|e| format!("Input emulation request cancelled or denied: {}", e))?;
            
            let i_token = selected_devices.restore_token()
                .map(|t| t.to_string())
                .or(Some("session".to_string()));

            {
                let mut config = state.config.lock().unwrap();
                config.input_token = i_token;
                let _ = crate::config::save_config(&config);
            }
        }
        return Ok(());
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(())
    }
}

// Tauri commands
#[tauri::command]
async fn log_ui_event(message: String) {
    log_info!("[UI] {}", message);
}

#[tauri::command]
async fn check_hotkey_status(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let error = state.hotkey_error.lock().unwrap();
    Ok(error.clone())
}

#[tauri::command]
async fn manual_register_hotkey(
    new_hotkey: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    log_info!("Manual hotkey registration requested for: {}", new_hotkey);
    
    // Save to config first
    {
        let mut config = state.config.lock().unwrap();
        config.hotkey = new_hotkey.clone();
        let _ = crate::config::save_config(&config);
    }

    let backend = state.display_backend.clone();
    backend.start_engine(app_handle, true).await
}

#[tauri::command]
async fn get_audio_devices() -> Result<Vec<audio::AudioDevice>, String> {
    log_info!("📡 Tauri Command: get_audio_devices invoked");
    audio::get_input_devices()
}

#[tauri::command]
async fn set_configuring_hotkey(
    is_configuring: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut config_flag = state.is_configuring_hotkey.lock().unwrap();
    *config_flag = is_configuring;
    log_info!("🔧 set_configuring_hotkey: {}", is_configuring);
    Ok(())
}

#[tauri::command]
async fn start_recording(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    if *state.is_configuring_hotkey.lock().unwrap() {
        log_info!("⚠️ Ignoring start_recording because hotkey configuration is active");
        return Err("Currently configuring hotkey".to_string());
    }

    let mut recording_flag = state.is_recording.lock().unwrap();
    if *recording_flag {
        return Err("Already recording".to_string());
    }
    
    *recording_flag = true;
    log_info!("🎤 start_recording command - Flag set to true immediately");

    let is_recording_clone = state.is_recording.clone();
    let config = state.config.clone();
    let app_handle_clone = app_handle.clone();
    let audio_engine = state.audio_engine.clone();
    let virtual_keyboard = state.virtual_keyboard.clone();

    // Ensure engine is initialized
    {
        let mut engine_guard = audio_engine.lock().unwrap();
        if engine_guard.is_none() {
            log_info!("🔧 Audio engine not found, attempting to initialize...");
            let cached_device = state.cached_device.lock().unwrap().clone();
            if let Some(dev) = cached_device {
                let sensitivity = config.lock().unwrap().input_sensitivity;
                if let Ok(new_eng) = audio::PersistentAudioEngine::new(&dev, sensitivity) {
                    *engine_guard = Some(new_eng);
                    log_info!("✅ Audio engine initialized on demand");
                }
            }
        }
    }

    tokio::spawn(async move {
        emit_status_update("Recording").await;
        let result = record_and_transcribe(config, is_recording_clone, app_handle_clone, audio_engine, virtual_keyboard).await;
        
        if let Err(e) = result {
            log_info!("❌ Global Recording error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
async fn stop_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut recording = state.is_recording.lock().unwrap();
    *recording = false;
    log_info!("⏹️  stop_recording command - Flag set to false");
    Ok(())
}

#[tauri::command]
async fn start_mic_test(state: tauri::State<'_, AppState>, app_handle: tauri::AppHandle) -> Result<(), String> {
    log_info!("📡 Tauri Command: start_mic_test invoked");
    let mut mic_test_flag = state.is_mic_test_active.lock().unwrap();
    if *mic_test_flag {
        log_info!("⚠️  start_mic_test: Already active");
        return Err("Mic test already active".to_string());
    }
    *mic_test_flag = true;
    
    // Clear previous samples
    let mut samples = state.mic_test_samples.lock().unwrap();
    samples.clear();
    
    let is_mic_test_clone = state.is_mic_test_active.clone();
    let mic_test_samples_clone = state.mic_test_samples.clone();
    let audio_engine = state.audio_engine.clone();
    let playback_stream_state = state.playback_stream.clone();
    let app_handle_clone = app_handle.clone();
    
    // Ensure engine is initialized
    {
        let mut engine_guard = audio_engine.lock().unwrap();
        if engine_guard.is_none() {
            log_info!("🔧 Audio engine not found for mic test, attempting to initialize...");
            let cached_device = state.cached_device.lock().unwrap().clone();
            if let Some(dev) = cached_device {
                let sensitivity = state.config.lock().unwrap().input_sensitivity;
                if let Ok(new_eng) = audio::PersistentAudioEngine::new(&dev, sensitivity) {
                    *engine_guard = Some(new_eng);
                    log_info!("✅ Audio engine initialized on demand");
                }
            }
        }
    }

    tokio::spawn(async move {
        log_info!("🎤 Mic test thread started");
        
        let sample_rate = {
            let guard = audio_engine.lock().unwrap();
            guard.as_ref().map(|e| e.sample_rate).unwrap_or(16000)
        };

        let result = audio::record_mic_test(&is_mic_test_clone, audio_engine, {

            let app = app_handle_clone.clone();
            move |volume| {
                let _ = app.emit("mic-test-volume", volume);
            }
        }).await;

        match result {
            Ok(captured_samples) => {
                log_info!("✅ Mic test captured {} samples", captured_samples.len());
                if captured_samples.is_empty() {
                    log_info!("⚠️  No audio captured, resetting UI...");
                    let _ = app_handle_clone.emit("mic-test-playback-finished", ());
                    return;
                }

                // Restore Playback Logic
                log_info!("🔊 Initializing playback at {}Hz...", sample_rate);
                let app = app_handle_clone.clone();
                match audio::play_audio(captured_samples.clone(), sample_rate, move || {
                    log_info!("🎵 Mic test playback finished");
                    let _ = app.emit("mic-test-playback-finished", ());
                }) {

                    Ok(stream) => {
                        let mut stream_guard = playback_stream_state.lock().unwrap();
                        *stream_guard = Some(stream);
                        log_info!("✅ Playback stream active");
                        let _ = app_handle_clone.emit("mic-test-playback-started", ());
                    }
                    Err(e) => {
                        log_info!("❌ Playback stream initialization failed: {}", e);
                        let _ = app_handle_clone.emit("mic-test-playback-finished", ());
                    }
                }
                
                let mut samples = mic_test_samples_clone.lock().unwrap();
                *samples = captured_samples;
            }
            Err(e) => {
                log_info!("❌ Mic test recording error: {}", e);
                let _ = app_handle_clone.emit("mic-test-playback-finished", ());
            }
        }
    });
    
    Ok(())
}

#[tauri::command]
async fn stop_mic_test(state: tauri::State<'_, AppState>) -> Result<(), String> {
    log_info!("📡 Tauri Command: stop_mic_test invoked");
    let mut mic_test_flag = state.is_mic_test_active.lock().unwrap();
    *mic_test_flag = false;
    log_info!("⏹️  Mic test flag set to false");
    Ok(())
}

#[tauri::command]
async fn stop_mic_playback(state: tauri::State<'_, AppState>) -> Result<(), String> {
    log_info!("📡 Tauri Command: stop_mic_playback invoked");
    let mut stream_guard = state.playback_stream.lock().unwrap();
    *stream_guard = None; // Dropping the stream stops playback
    log_info!("⏹️  Playback stopped by user");
    Ok(())
}

#[tauri::command]
async fn open_debug_folder() -> Result<(), String> {
    log_info!("📡 Tauri Command: open_debug_folder invoked");
    let path = dirs::config_dir()
        .ok_or("Could not find config directory")?
        .join("voquill")
        .join("debug");
    
    log_info!("📂 Target debug path: {:?}", path);
    
    if !path.exists() {
        log_info!("📂 Creating debug directory...");
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }
    
    #[cfg(target_os = "linux")]
    {
        log_info!("🚀 Executing: xdg-open {:?}", path);
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| {
                log_info!("❌ Failed to execute xdg-open: {}", e);
                e.to_string()
            })?;
    }
    #[cfg(target_os = "windows")]
    {
        log_info!("🚀 Executing: explorer {:?}", path);
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| {
                log_info!("❌ Failed to execute explorer: {}", e);
                e.to_string()
            })?;
    }
    #[cfg(target_os = "macos")]
    {
        log_info!("🚀 Executing: open {:?}", path);
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| {
                log_info!("❌ Failed to execute open: {}", e);
                e.to_string()
            })?;
    }
    
    Ok(())
}

#[tauri::command]
async fn get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
async fn save_config(
    new_config: Config,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let (restart_engine, hotkey_changed) = {
        let mut config = state.config.lock().unwrap();
        let audio_changed = config.audio_device != new_config.audio_device || config.input_sensitivity != new_config.input_sensitivity;
        let hotkey_changed = config.hotkey != new_config.hotkey;
        
        // CRITICAL: Preserve internal tokens that the frontend doesn't manage
        let mut merged_config = new_config.clone();
        if merged_config.shortcuts_token.is_none() {
            merged_config.shortcuts_token = config.shortcuts_token.clone();
        }
        if merged_config.input_token.is_none() {
            merged_config.input_token = config.input_token.clone();
        }
        
        *config = merged_config;

        // Pre-warm the audio device cache
        let mut cached_device = state.cached_device.lock().unwrap();
        *cached_device = audio::lookup_device(config.audio_device.clone()).ok();
        log_info!("🔧 Pre-warmed audio device cache");
        
        (audio_changed, hotkey_changed)
    };

    if restart_engine {
        log_info!("🔧 Audio config changed, restarting persistent engine...");
        let cached_device = state.cached_device.lock().unwrap().clone();
        let sensitivity = new_config.input_sensitivity;
        let mut engine_guard = state.audio_engine.lock().unwrap();
        *engine_guard = None; // Drop old stream
        if let Some(dev) = cached_device {
            if let Ok(new_eng) = audio::PersistentAudioEngine::new(&dev, sensitivity) {
                *engine_guard = Some(new_eng);
                log_info!("✅ Persistent engine restarted");
            }
        }
    }
    
    if let Err(e) = config::save_config(&new_config) {
        let error_msg = format!("Failed to save config: {}", e);
        return Err(error_msg);
    }
    
    if hotkey_changed {
        if let Err(e) = re_register_hotkey(&app_handle, &new_config.hotkey).await {
            let mut error_lock = state.hotkey_error.lock().unwrap();
            *error_lock = Some(e.clone());
            return Err(format!("Config saved but failed to update hotkey: {}", e));
        } else {
            let mut error_lock = state.hotkey_error.lock().unwrap();
            *error_lock = None;
        }
    }
    
    Ok(())
}

async fn re_register_hotkey(app_handle: &tauri::AppHandle, hotkey_string: &str) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    log_info!("🔄 Re-registering hotkey '{}'...", hotkey_string);
    
    let app_handle_clone = app_handle.clone();
    let backend = state.display_backend.clone();
    tauri::async_runtime::spawn(async move {
        backend.start_engine(app_handle_clone, false).await;
    });

    Ok(())
}

#[tauri::command]
async fn test_api_key(api_key: String, api_url: String) -> Result<bool, String> {
    transcription::test_api_key(&api_key, &api_url).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_available_engines() -> Result<Vec<String>, String> {
    Ok(model_manager::ModelManager::get_available_engines())
}

#[tauri::command]
async fn get_available_models() -> Result<Vec<model_manager::ModelInfo>, String> {
    Ok(model_manager::ModelManager::get_available_models())
}

#[tauri::command]
async fn check_model_status(model_size: String) -> Result<bool, String> {
    let mm = model_manager::ModelManager::new().map_err(|e| e.to_string())?;
    Ok(mm.is_model_downloaded(&model_size))
}

#[tauri::command]
async fn download_model(model_size: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let mm = model_manager::ModelManager::new().map_err(|e| e.to_string())?;
    
    mm.download_model(&model_size, move |progress| {
        let _ = app_handle.emit("model-download-progress", progress);
    }).await?;
    
    Ok(())
}

static CURRENT_STATUS: OnceLock<Mutex<String>> = OnceLock::new();

#[tauri::command]
fn get_current_status() -> String {
    if let Some(status_mutex) = CURRENT_STATUS.get() {
        if let Ok(status) = status_mutex.lock() {
            return status.clone();
        }
    }
    "Ready".to_string()
}

#[tauri::command]
async fn get_history() -> Result<history::History, String> {
    history::load_history().map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_history() -> Result<(), String> {
    history::clear_history().map_err(|e| e.to_string())
}

async fn hide_overlay_window(app_handle: &AppHandle) -> Result<(), String> {
    if let Some(overlay_window) = app_handle.get_webview_window("overlay") {
        overlay_window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn position_overlay_window(overlay_window: &WebviewWindow, app_handle: &AppHandle) -> Result<(), String> {
    let app_state = app_handle.state::<AppState>();
    let pixels_from_bottom_logical = {
        let config = app_state.config.lock().unwrap();
        config.pixels_from_bottom
    };
    
    app_state.display_backend.position_overlay_window(overlay_window, pixels_from_bottom_logical)?;
    Ok(())
}


async fn show_overlay_window(app_handle: &AppHandle) -> Result<(), String> {
    log_info!("🔍 show_overlay_window called");
    let overlay_window = app_handle.get_webview_window("overlay").ok_or("Overlay window not found")?;
    
    if overlay_window.is_visible().unwrap_or(false) {
        log_info!("🔍 Overlay already visible");
        return Ok(());
    }

    log_info!("🔍 Positioning and showing overlay...");
    position_overlay_window(&overlay_window, app_handle).await?;
    
    // Use Tauri native show() to maintain reference count stability
    overlay_window.show().map_err(|e| e.to_string())?;
    
    // Ghost Mode handled by display backend
    let state = app_handle.state::<AppState>();
    state.display_backend.apply_overlay_hints(&overlay_window);
    
    log_info!("✅ Overlay visibility commanded");
    Ok(())
}

// Centralized status emitter
async fn emit_status_update(status: &str) {
    let mut changed = false;
    if let Some(status_mutex) = CURRENT_STATUS.get() {
        if let Ok(mut global_status) = status_mutex.lock() {
            if *global_status != status {
                *global_status = status.to_string();
                changed = true;
            }
        }
    }
    
    if !changed {
        return;
    }
    
    log_info!("🔄 App Status Change: {}", status);
    
    if let Some(app_handle) = APP_HANDLE.get() {
        let windows = ["main", "overlay"];
        for window_label in &windows {
            if let Some(window) = app_handle.get_webview_window(window_label) {
                let _ = window.emit("status-update", status);
            }
        }
        
        if status == "Ready" || status == "Typing" {
            let _ = hide_overlay_window(app_handle).await;
        } else {
            let _ = show_overlay_window(app_handle).await;
        }
    }
}

async fn emit_status_to_frontend(status: &str) {
    emit_status_update(status).await;
}

fn validate_audio_duration(audio_data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if audio_data.len() < 44 { return Err("Audio file too small".into()); }
    let sample_rate = u32::from_le_bytes([audio_data[24], audio_data[25], audio_data[26], audio_data[27]]);
    let channels = u16::from_le_bytes([audio_data[22], audio_data[23]]);
    let bits_per_sample = u16::from_le_bytes([audio_data[34], audio_data[35]]);
    
    let mut data_size = 0u32;
    let mut pos = 36;
    while pos + 8 <= audio_data.len() {
        let chunk_id = &audio_data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([audio_data[pos + 4], audio_data[pos + 5], audio_data[pos + 6], audio_data[pos + 7]]);
        if chunk_id == b"data" { data_size = chunk_size; break; }
        pos += 8 + chunk_size as usize;
        if chunk_size % 2 == 1 { pos += 1; }
    }
    
    if data_size == 0 { return Err("No data chunk".into()); }
    let bytes_per_sample = (bits_per_sample / 8) as u32;
    let bytes_per_second = sample_rate * channels as u32 * bytes_per_sample;
    let duration_seconds = data_size as f64 / bytes_per_second as f64;
    
    log_info!("Audio duration: {:.3}s", duration_seconds);
    if duration_seconds < 0.1 { return Err("Audio too short".into()); }
    Ok(())
}

async fn record_and_transcribe(
    config: Arc<Mutex<Config>>,
    is_recording: Arc<Mutex<bool>>,
    app_handle: AppHandle,
    audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
    virtual_keyboard: Arc<Mutex<Option<VirtualKeyboardHandle>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reset_status_on_exit = || async { emit_status_to_frontend("Ready").await; };
    
    let audio_data = match audio::record_audio_while_flag(&is_recording, audio_engine).await {
        Ok(data) => data,
        Err(e) => {
            reset_status_on_exit().await;
            return Err(e);
        }
    };
    
    if audio_data.is_empty() { reset_status_on_exit().await; return Ok(()); }
    if let Err(e) = validate_audio_duration(&audio_data) { 
        log_info!("⚠️ Audio validation failed: {}", e);
        reset_status_on_exit().await; 
        return Ok(()); 
    }
    
    emit_status_to_frontend("Transcribing").await;
    let (transcription_mode, api_key, api_url, api_model, debug_mode, enable_recording_logs, language_choice) = {
        let config_guard = config.lock().unwrap();
        (
            config_guard.transcription_mode.clone(),
            config_guard.openai_api_key.clone(),
            config_guard.api_url.clone(),
            config_guard.api_model.clone(),
            config_guard.debug_mode,
            config_guard.enable_recording_logs,
            config_guard.language.clone(),
        )
    };

    let (lang_code, prompt_hint) = match language_choice.as_str() {
        "auto" => (None, None),
        "en-AU" => (Some("en"), Some("Australian spelling.")),
        "en-GB" => (Some("en"), Some("British spelling.")),
        "en-US" => (Some("en"), Some("American spelling.")),
        code => (Some(code), None),
    };
    
    if debug_mode && enable_recording_logs {

        let debug_path = dirs::config_dir()
            .unwrap_or_default()
            .join("voquill")
            .join("debug")
            .join(format!("recording_{}.wav", ::chrono::Local::now().format("%Y%m%d_%H%M%S")));
        
        if let Err(e) = std::fs::create_dir_all(debug_path.parent().unwrap()) {
            log_info!("❌ Failed to create debug directory: {}", e);
        } else if let Err(e) = std::fs::write(&debug_path, &audio_data) {
            log_info!("❌ Failed to save debug recording: {}", e);
        } else {
            log_info!("🛡️ Debug recording saved to: {:?}", debug_path);
        }
    }
    
    log_info!("📡 Transcription Mode: {:?}", transcription_mode);
    log_info!("🌐 Language: {:?}, Hint: {:?}", lang_code, prompt_hint);
    
    let service: Box<dyn transcription::TranscriptionService + Send + Sync> = match transcription_mode {
        TranscriptionMode::API => Box::new(transcription::APITranscriptionService {
            api_key,
            api_url,
            api_model,
        }),
        TranscriptionMode::Local => {
            let (model_size, use_gpu) = {
                let config_lock = config.lock().unwrap();
                (config_lock.local_model_size.clone(), config_lock.enable_gpu)
            };
            match local_whisper::LocalWhisperService::new(&model_size, use_gpu) {
                Ok(s) => Box::new(s),
                Err(e) => {
                    log_info!("❌ Failed to initialize Local Whisper: {}", e);
                    reset_status_on_exit().await;
                    return Err(e.into());
                }
            }
        }
    };

    let text = match service.transcribe(&audio_data, lang_code, prompt_hint).await {
        Ok(text) => {
            log_info!("📝 Transcription received ({}): \"{}\"", service.service_name(), text);
            text
        },

        Err(e) => { 
            log_info!("❌ Transcription failed ({}): {}", service.service_name(), e);
            reset_status_on_exit().await; 
            return Err(e.into()); 
        }
    };
    
    if !text.trim().is_empty() {
        let _ = history::add_history_item(&text);
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("history-updated", ());
        }
        
        emit_status_to_frontend("Typing").await;
        let (typing_speed, hold_duration, output_method, copy_on_typewriter) = { 
            let config_guard = config.lock().unwrap();
            (
                config_guard.typing_speed_interval, 
                config_guard.key_press_duration_ms,
                config_guard.output_method.clone(),
                config_guard.copy_on_typewriter
            ) 
        };
        
        // Give the OS a moment to ensure the overlay is hidden and focus is restored to the target app
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        
        match output_method {
            config::OutputMethod::Typewriter => {
                if copy_on_typewriter {
                    if let Err(e) = typing::copy_to_clipboard(&text) {
                        log_info!("❌ CLIPBOARD ERROR: {}", e);
                    }
                }
                log_info!("⌨️  Forwarding text to hardware typing engine...");
                let state = app_handle.state::<AppState>();
                if let Err(e) = state.display_backend.type_text_hardware(&text, typing_speed, hold_duration, virtual_keyboard) {
                    log_info!("❌ TYPING ENGINE ERROR: {}", e);
                }
            },
            config::OutputMethod::Clipboard => {
                log_info!("📋 Copying text to clipboard (Clipboard Mode)...");
                if let Err(e) = typing::copy_to_clipboard(&text) {
                    log_info!("❌ CLIPBOARD ERROR: {}", e);
                }
            }
        }
    } else {
        log_info!("ℹ️ Transcription was empty, skipping typing.");
    }
    
    reset_status_on_exit().await;
    Ok(())
}


fn main() {
    #[cfg(target_os = "linux")]
    {
        enforce_wayland();
    }

    env_logger::init();

    let _is_first_launch = config::is_first_launch().unwrap_or(false);
    let initial_config = config::load_config().unwrap_or_default();
    
    let app_state = AppState {
        config: Arc::new(Mutex::new(initial_config.clone())),
        hardware_hotkey: Arc::new(Mutex::new(hotkey::parse_hardware_hotkey(&initial_config.hotkey))),
        ..Default::default()
    };

    {
        let mut cached_device = app_state.cached_device.lock().unwrap();
        let dev = audio::lookup_device(initial_config.audio_device.clone()).ok();
        *cached_device = dev.clone();
        
        if let Some(d) = dev {
            if let Ok(engine) = audio::PersistentAudioEngine::new(&d, initial_config.input_sensitivity) {
                let mut engine_guard = app_state.audio_engine.lock().unwrap();
                *engine_guard = Some(engine);
                log_info!("✅ Persistent audio engine initialized");
            }
        }
        log_info!("🔧 Initial pre-warm of audio device cache complete");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, _shortcut, event| {
                // Ignore plugin hotkeys on Wayland, use Portal instead
                if std::env::var("WAYLAND_DISPLAY").is_ok() {
                    return;
                }
                if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<AppState>();
                        let _ = start_recording(state, app_handle.clone()).await;
                    });
                } else {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<AppState>();
                        let _ = stop_recording(state).await;
                    });
                }
            })
            .build())
        .manage(app_state)
        .setup(move |app| {
            let _ = APP_HANDLE.set(app.handle().clone());
            let _ = CURRENT_STATUS.set(Mutex::new("Ready".to_string()));

            #[cfg(target_os = "linux")]
            {
                check_wayland_display();
            }
            
            if let Some(w) = app.get_webview_window("overlay") { 
                log_info!("🔍 Overlay window found in setup");
                let _ = w.hide(); 
                let state = app.state::<AppState>();
                state.display_backend.apply_overlay_hints(&w);
            } else {
                log_info!("❌ Overlay window NOT FOUND in setup!");
            }
            let _ = audio::get_input_devices();
            
            #[cfg(target_os = "linux")]
            {
                let state = app.state::<AppState>();
                let virtual_keyboard = state.virtual_keyboard.clone();
                std::thread::spawn(move || {
                    use evdev::uinput::VirtualDevice;
                    use evdev::{AttributeSet, KeyCode, InputId, BusType};
                    log_info!("🔄 Starting virtual hardware keyboard initialization...");
                    
                    let mut keys = AttributeSet::<KeyCode>::new();
                    for i in 0..564 {
                        keys.insert(KeyCode::new(i as u16));
                    }

                    let input_id = InputId::new(BusType::BUS_USB, 0x6666, 0x8888, 0x0111);

                    match VirtualDevice::builder()
                        .map_err(|e| e.to_string())
                        .and_then(|b| b.name("Voquill Virtual Keyboard")
                                     .input_id(input_id)
                                     .with_keys(&keys)
                                     .map_err(|e| e.to_string()))
                        .and_then(|b| b.build().map_err(|e| e.to_string()))
                    {
                        Ok(mut device) => {
                            if let Ok(path) = device.get_syspath() {
                                log_info!("✅ Virtual hardware keyboard initialized at: {}", path.display());
                            } else {
                                log_info!("✅ Virtual hardware keyboard initialized");
                            }
                            let mut lock = virtual_keyboard.lock().unwrap();
                            *lock = Some(device);
                        },
                        Err(e) => {
                            log_info!("❌ Virtual keyboard initialization failed: {}. Input emulation may not work.", e);
                        }
                    }
                });
            }

            let menu = create_tray_menu(app.handle())?;
            let _tray = TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app_handle, event| match event.id.as_ref() {
                    "quit" => { std::process::exit(0); }
                    "open" => { if let Some(w) = app_handle.get_webview_window("main") { let _ = w.show(); let _ = w.set_focus(); } }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            if let Some(w) = app.get_webview_window("main") {
                let w_c = w.clone();
                w.on_window_event(move |event| { if let tauri::WindowEvent::CloseRequested { api, .. } = event { api.prevent_close(); let _ = w_c.hide(); } });
            }

            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }

            // Initial hotkey registration
            let hotkey_string = initial_config.hotkey.clone();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let _ = re_register_hotkey(&app_handle, &hotkey_string).await;
            });

            #[cfg(target_os = "linux")]
            {
                // No automatic setup anymore
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording, stop_recording, get_config, save_config,
            test_api_key, get_current_status, get_history, clear_history,
            check_hotkey_status, manual_register_hotkey, get_audio_devices,
            start_mic_test, stop_mic_test, stop_mic_playback, open_debug_folder,
            log_ui_event, get_available_engines, get_available_models, check_model_status, download_model,
            get_linux_setup_status, request_audio_permission, request_input_permission, set_configuring_hotkey, get_wayland_portal_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn create_tray_menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let open_i = MenuItem::with_id(app, "open", "Open Voquill", true, None::<&str>)?;
    Menu::with_items(app, &[&open_i, &quit_i])
}
