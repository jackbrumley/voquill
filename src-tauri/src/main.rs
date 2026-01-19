// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Logging macro with timestamps
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        println!("[{}] {}", ::chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"), format!($($arg)*))
    };
}

use std::sync::{Arc, Mutex, OnceLock};
#[cfg(target_os = "linux")]
use std::collections::HashSet;
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

use config::{Config, TranscriptionMode};
use hotkey::HardwareHotkey;

#[cfg(target_os = "linux")]
async fn check_request_audio_portal(app_handle: &tauri::AppHandle) -> Result<(), String> {
    use ashpd::desktop::camera::Camera;
    
    log_info!("Checking Audio/Microphone portal status (via Camera portal proxy)...");
    
    match Camera::new().await {
        Ok(proxy) => {
            match proxy.request_access().await {
                Ok(_request) => {
                    log_info!("✅ Audio/Microphone portal request sent");
                    Ok(())
                },
                Err(e) => {
                    let error_msg = format!("{}", e);
                    log_info!("⚠️ Audio/Microphone portal request failed: {}", error_msg);
                    if !error_msg.contains("not found") {
                        let _ = app_handle.emit("audio-error", "portal-denied");
                    }
                    Ok(()) 
                }
            }
        },
        Err(e) => {
            log_info!("⚠️ Audio/Microphone portal not available ({}). PulseAudio policy will manage access.", e);
            Ok(())
        }
    }
}

// Application state
#[cfg(target_os = "linux")]
pub type VirtualKeyboardHandle = evdev::uinput::VirtualDevice;
#[cfg(not(target_os = "linux"))]
pub type VirtualKeyboardHandle = ();

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub is_recording: Arc<Mutex<bool>>,
    pub is_mic_test_active: Arc<Mutex<bool>>,
    pub hotkey_error: Arc<Mutex<Option<String>>>,
    pub setup_status: Arc<Mutex<Option<String>>>,
    pub hardware_hotkey: Arc<Mutex<HardwareHotkey>>,
    pub cached_device: Arc<Mutex<Option<cpal::Device>>>,
    pub virtual_keyboard: Arc<Mutex<Option<VirtualKeyboardHandle>>>,
    pub playback_stream: Arc<Mutex<Option<cpal::Stream>>>,
    pub mic_test_samples: Arc<Mutex<Vec<f32>>>,
    pub audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Arc::new(Mutex::new(Config::default())),
            is_recording: Arc::new(Mutex::new(false)),
            is_mic_test_active: Arc::new(Mutex::new(false)),
            hotkey_error: Arc::new(Mutex::new(None)),
            setup_status: Arc::new(Mutex::new(None)),
            hardware_hotkey: Arc::new(Mutex::new(HardwareHotkey::default())),
            cached_device: Arc::new(Mutex::new(None)),
            virtual_keyboard: Arc::new(Mutex::new(None)),
            playback_stream: Arc::new(Mutex::new(None)),
            mic_test_samples: Arc::new(Mutex::new(Vec::new())),
            audio_engine: Arc::new(Mutex::new(None)),
        }
    }
}

#[cfg(target_os = "linux")]
#[tauri::command]
async fn get_linux_setup_status() -> Result<bool, String> {
    use std::fs;
    use std::process::Command;

    // 1. Check uinput access directly
    let has_uinput_access = fs::OpenOptions::new().write(true).open("/dev/uinput").is_ok();
    
    // 2. Check group memberships
    let groups_output = match Command::new("groups").output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(_) => return Ok(false),
    };
    
    let is_in_audio = groups_output.contains("audio");
    let is_in_input = groups_output.contains("input");

    Ok(has_uinput_access && is_in_audio && is_in_input)
}

#[cfg(target_os = "linux")]
#[tauri::command]
async fn run_linux_setup(app_handle: tauri::AppHandle) -> Result<(), String> {
    use std::process::Command;

    log_info!("🚀 User initiated Linux system setup...");
    
    // Trigger portal request first
    let _ = check_request_audio_portal(&app_handle).await;

    let groups_output = match Command::new("groups").output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(e) => return Err(format!("Failed to check groups: {}", e)),
    };

    let username = std::env::var("USER").unwrap_or_default();
    if username.is_empty() {
        return Err("Could not determine username".to_string());
    }

    let is_in_uinput = groups_output.contains("uinput");
    let uinput_group = if is_in_uinput { "uinput" } else { "input" };
    
    let cmd = format!("usermod -aG audio,input,{} {}", uinput_group, username);
    log_info!("🔧 Executing: pkexec {}", cmd);
    
    let output = Command::new("pkexec")
        .args(["bash", "-c", &cmd])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            log_info!("✅ Permissions updated successfully.");
            let _ = app_handle.emit("setup-status", "restart-required");
            Ok(())
        },
        _ => {
            log_info!("⚠️ Permission update failed or cancelled.");
            Err("Setup failed or cancelled by user".to_string())
        }
    }
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
async fn get_linux_setup_status() -> Result<bool, String> { Ok(true) }

#[cfg(not(target_os = "linux"))]
#[tauri::command]
async fn run_linux_setup() -> Result<(), String> { Ok(()) }

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
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let hotkey_string = {
        let config = state.config.lock().unwrap();
        config.hotkey.clone()
    };
    
    log_info!("Manual hotkey registration requested: {}", hotkey_string);
    
    if let Err(e) = re_register_hotkey(&app_handle, &hotkey_string).await {
        let mut error_lock = state.hotkey_error.lock().unwrap();
        *error_lock = Some(e.clone());
        return Err(e);
    } else {
        let mut error_lock = state.hotkey_error.lock().unwrap();
        *error_lock = None;
    }
    
    Ok(())
}

#[tauri::command]
async fn get_audio_devices() -> Result<Vec<audio::AudioDevice>, String> {
    log_info!("📡 Tauri Command: get_audio_devices invoked");
    audio::get_input_devices()
}

#[tauri::command]
async fn start_recording(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
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
    let mut restart_engine = false;
    {
        let mut config = state.config.lock().unwrap();
        if config.audio_device != new_config.audio_device || config.input_sensitivity != new_config.input_sensitivity {
            restart_engine = true;
        }
        *config = new_config.clone();
        
        // Update Hardware Hotkey for Linux
        #[cfg(target_os = "linux")]
        {
            let mut hardware_hotkey = state.hardware_hotkey.lock().unwrap();
            *hardware_hotkey = hotkey::parse_hardware_hotkey(&new_config.hotkey);
            log_info!("🔧 Updated hardware hotkey: {:?}", *hardware_hotkey);
        }

        // Pre-warm the audio device cache
        let mut cached_device = state.cached_device.lock().unwrap();
        *cached_device = audio::lookup_device(new_config.audio_device.clone()).ok();
        log_info!("🔧 Pre-warmed audio device cache");
    }

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
    
    if let Err(e) = re_register_hotkey(&app_handle, &new_config.hotkey).await {
        let mut error_lock = state.hotkey_error.lock().unwrap();
        *error_lock = Some(e.clone());
        return Err(format!("Config saved but failed to update hotkey: {}", e));
    } else {
        let mut error_lock = state.hotkey_error.lock().unwrap();
        *error_lock = None;
    }
    
    Ok(())
}

async fn re_register_hotkey(_app_handle: &tauri::AppHandle, _hotkey_string: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        log_info!("Re-registering hotkey: {}", _hotkey_string);
        let _ = _app_handle.global_shortcut().unregister_all();
        match hotkey::parse_hotkey_string(_hotkey_string) {
            Ok(shortcut) => {
                _app_handle.global_shortcut().register(shortcut).map_err(|e| e.to_string())?;
                log_info!("✅ Global hotkey registered: {}", _hotkey_string);
                Ok(())
            }
            Err(e) => Err(e.to_string())
        }
    }
}

#[tauri::command]
async fn test_api_key(api_key: String, api_url: String) -> Result<bool, String> {
    transcription::test_api_key(&api_key, &api_url).await.map_err(|e| e.to_string())
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
    
    #[cfg(target_os = "linux")]
    {
        use gtk_layer_shell::LayerShell;
        let window_clone = overlay_window.clone();
        
        // Dispatch to main thread for GTK operations
        gtk::glib::MainContext::default().invoke(move || {
            if let Ok(gtk_window) = window_clone.gtk_window() {
                // Resolution: Use fully qualified syntax and the correct method name 'set_layer_shell_margin'
                // to resolve the conflict with WidgetExt::set_margin
                gtk_window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
                LayerShell::set_layer_shell_margin(&gtk_window, gtk_layer_shell::Edge::Bottom, pixels_from_bottom_logical);
            }
        });
        
        Ok(())
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        use tauri::Position;
        // Standardized: Always use the OS-reported Primary Monitor for the status overlay.
        let monitor = overlay_window.primary_monitor()
            .map_err(|e| e.to_string())?
            .or_else(|| overlay_window.available_monitors().ok().and_then(|m| m.first().cloned()))
            .ok_or("No monitors found")?;
        
        let monitor_size = monitor.size();
        let monitor_position = monitor.position();
        let scale_factor = monitor.scale_factor();
        
        // Physical Pixel calculations for high-DPI accuracy
        let pixels_from_bottom_physical = (pixels_from_bottom_logical as f64 * scale_factor) as i32;
        let window_width_logical = 140.0;
        let window_height_logical = 140.0;
        
        let window_width_physical = (window_width_logical * scale_factor) as i32;
        let window_height_physical = (window_height_logical * scale_factor) as i32;
        
        let x = monitor_position.x + (monitor_size.width as i32 - window_width_physical) / 2;
        let y = monitor_position.y + monitor_size.height as i32 - window_height_physical - pixels_from_bottom_physical;
        
        log_info!("📍 Positioning overlay at Physical: {}, {} (Monitor: {:?}x{:?} at {:?}, Scale: {})", 
            x, y, monitor_size.width, monitor_size.height, monitor_position, scale_factor);
        
        overlay_window.set_position(Position::Physical(tauri::PhysicalPosition::new(x, y))).map_err(|e| e.to_string())?;
        overlay_window.set_size(tauri::LogicalSize::new(window_width_logical, window_height_logical)).map_err(|e| e.to_string())?;
        
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn apply_linux_unfocusable_hints(window: &WebviewWindow) {
    use gtk::prelude::*;
    use gtk_layer_shell::LayerShell;

    if let Ok(gtk_window) = window.gtk_window() {
        log_info!("🛠️  Initializing Wayland Layer Shell for overlay...");

        gtk_window.init_layer_shell();
        gtk_window.set_layer(gtk_layer_shell::Layer::Overlay);
        gtk_window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
        
        // Use set_keyboard_mode (requires v0_6 feature)
        gtk_window.set_keyboard_mode(gtk_layer_shell::KeyboardMode::None);

        // Set initial margin from config
        let app_handle = window.app_handle();
        let app_state = app_handle.state::<AppState>();
        let pixels_from_bottom_logical = {
            let config = app_state.config.lock().unwrap();
            config.pixels_from_bottom
        };
        LayerShell::set_layer_shell_margin(&gtk_window, gtk_layer_shell::Edge::Bottom, pixels_from_bottom_logical);

        // Standard GTK properties
        gtk_window.set_decorated(false);
        gtk_window.set_skip_taskbar_hint(true);
        gtk_window.set_skip_pager_hint(true);
    }
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
    
    // Ghost Mode: Ensure it never takes focus or blocks clicks
    // Only applied on non-linux platforms to avoid crashes with Layer Shell
    #[cfg(not(target_os = "linux"))]
    {
        let overlay_clone = overlay_window.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            log_info!("👻 Applying Ghost Mode attributes...");
            let _ = overlay_clone.set_focusable(false);
            let _ = overlay_clone.set_ignore_cursor_events(true);
        });
    }
    
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
            let model_size = {
                let config_lock = config.lock().unwrap();
                config_lock.local_model_size.clone()
            };
            match local_whisper::LocalWhisperService::new(&model_size) {
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
                if let Err(e) = typing::type_text_hardware(&text, typing_speed, hold_duration, virtual_keyboard) {
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

#[cfg(target_os = "linux")]
fn start_linux_input_engine(app_handle: AppHandle) {
    use std::fs;
    use std::os::unix::io::AsRawFd;
    use libc::{poll, pollfd, POLLIN};
    
    let state = app_handle.state::<AppState>();
    let is_recording_flag = state.is_recording.clone();
    let hardware_hotkey_flag = state.hardware_hotkey.clone();

    std::thread::spawn(move || {
        log_info!("🚀 Linux Hardware Input Engine started.");
        
        let mut devices = Vec::new();
        if let Ok(entries) = fs::read_dir("/dev/input") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("event") {
                        if let Ok(device) = evdev::Device::open(&path) {
                            let has_keys = device.supported_keys().map(|k| k.iter().count() > 20).unwrap_or(false);
                            if has_keys {
                                let dev_name = device.name().unwrap_or("Unknown").to_string();
                                log_info!("🔍 Monitoring hardware: {} ({})", dev_name, path.display());
                                devices.push(device);
                            }
                        }
                    }
                }
            }
        }
        
        if devices.is_empty() {
            log_info!("⚠️ No keyboards found! Input engine disabled.");
            return;
        }

        let mut poll_fds: Vec<pollfd> = devices.iter().map(|d| pollfd { fd: d.as_raw_fd(), events: POLLIN, revents: 0 }).collect();
        let mut pressed_keys: HashSet<u16> = HashSet::new();

        loop {
            let ret = unsafe { poll(poll_fds.as_mut_ptr(), poll_fds.len() as libc::nfds_t, 100) };
            if ret < 0 { continue; }

            if ret > 0 {
                for i in 0..poll_fds.len() {
                    if poll_fds[i].revents & POLLIN != 0 {
                        let dev = &mut devices[i];
                        let dev_name = dev.name().unwrap_or("Unknown").to_string();
                        if let Ok(events) = dev.fetch_events() {
                            for event in events {
                                if let evdev::EventSummary::Key(_, key_code, value) = event.destructure() {
                                    let code = key_code.code();
                                    let h_hotkey = hardware_hotkey_flag.lock().unwrap().clone();

                                    if value == 1 { // Pressed
                                        log_info!("⌨️  [{}] Key {:?} PRESSED", dev_name, key_code);
                                        pressed_keys.insert(code);
                                        
                                        let all_pressed = !h_hotkey.all_codes.is_empty() && 
                                                         h_hotkey.all_codes.iter().all(|c| pressed_keys.contains(c));
                                        
                                        if all_pressed {
                                            let mut recording = is_recording_flag.lock().unwrap();
                                            if !*recording {
                                                *recording = true;
                                                log_info!("🎤 ENGINE: Combination Met! Starting recording.");
                                                
                                                let h_clone = app_handle.clone();
                                                tauri::async_runtime::spawn(async move {
                                                    emit_status_update("Recording").await;
                                                    let s = h_clone.state::<AppState>();
                                                    let config = s.config.clone();
                                                    let is_recording = s.is_recording.clone();
                                                    let audio_engine = s.audio_engine.clone();
                                                    let virtual_keyboard = s.virtual_keyboard.clone();
                                                    
                                                    let _ = record_and_transcribe(config, is_recording, h_clone, audio_engine, virtual_keyboard).await;
                                                });
                                                
                                                if let Some(w) = app_handle.get_webview_window("main") {
                                                    let _ = w.emit("hotkey-pressed", ());
                                                }
                                            }
                                        }
                                    } else if value == 0 { // Released
                                        log_info!("⌨️  [{}] Key {:?} RELEASED", dev_name, key_code);
                                        pressed_keys.remove(&code);
                                        
                                        let is_combo_key = h_hotkey.all_codes.contains(&code);
                                        let mut recording = is_recording_flag.lock().unwrap();
                                        if *recording && is_combo_key {
                                            *recording = false;
                                            log_info!("⏹️  ENGINE: Key Released! Finalizing.");
                                            
                                            let h_clone = app_handle.clone();
                                            tauri::async_runtime::spawn(async move {
                                                emit_status_update("Transcribing").await;
                                                if let Some(w) = h_clone.get_webview_window("main") {
                                                    let _ = w.emit("hotkey-released", ());
                                                }
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        // Pre-flight check: Ensure we are in a Wayland session
        // Voquill strictly requires Wayland for Layer Shell positioning and security protocols.
        let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
        
        if !is_wayland {
            let is_x11 = std::env::var("DISPLAY").is_ok();
            if is_x11 {
                eprintln!("\n\x1b[1;31m[Voquill Error] Wayland Session Required\x1b[0m");
                eprintln!("Voquill is built strictly for Wayland to ensure proper window positioning (via Layer Shell) and secure hardware access.");
                eprintln!("Your current session appears to be X11/XWayland, which is not supported.");
                eprintln!("Please log into a native Wayland session (GNOME, KDE, or Hyprland) to use this application.\n");
            } else {
                eprintln!("\n\x1b[1;31m[Voquill Error] No Wayland Display Detected\x1b[0m");
                eprintln!("Voquill requires a Wayland session to run. If you are in a Wayland session, ensure WAYLAND_DISPLAY is set.\n");
            }
            std::process::exit(1);
        }

        // Strictly Wayland: Enforce the Wayland backend for GTK.
        // This prevents fallbacks to XWayland/X11 which break Layer Shell positioning.
        std::env::set_var("GDK_BACKEND", "wayland");
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
                // Fix for Wayland taskbar icons: 
                // Set the program name to match the .desktop file name (voquill.desktop)
                // This allows the compositor to correctly associate the window with its icon.
                gtk::glib::set_prgname(Some("voquill"));
                gtk::glib::set_application_name("Voquill");

                // Diagnostic: Confirm we are running on Wayland
                if let Some(display) = gdk::Display::default() {
                    use gtk::glib::prelude::ObjectExt;
                    let type_name = display.type_().name();
                    let backend = if type_name.contains("Wayland") {
                        "Wayland ✅"
                    } else {
                        "X11/Unknown ❌ (Positioning will likely fail)"
                    };
                    log_info!("🖥️  GDK Backend: {} ({})", backend, type_name);
                }
            }
            
            if let Some(w) = app.get_webview_window("overlay") { 
                log_info!("🔍 Overlay window found in setup");
                let _ = w.hide(); 
                #[cfg(target_os = "linux")]
                apply_linux_unfocusable_hints(&w);
            } else {
                log_info!("❌ Overlay window NOT FOUND in setup!");
            }
            let _ = audio::get_input_devices();
            
            #[cfg(target_os = "linux")]
            start_linux_input_engine(app.handle().clone());

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
                        .unwrap()
                        .name("Voquill Virtual Keyboard")
                        .input_id(input_id)
                        .with_keys(&keys)
                        .unwrap()
                        .build() 
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
                            log_info!("❌ Virtual keyboard initialization failed: {}", e);
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
            log_ui_event, get_available_models, check_model_status, download_model,
            get_linux_setup_status, run_linux_setup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn create_tray_menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let open_i = MenuItem::with_id(app, "open", "Open Voquill", true, None::<&str>)?;
    Menu::with_items(app, &[&open_i, &quit_i])
}
