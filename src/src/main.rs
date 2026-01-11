// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex, OnceLock};
use tauri::{
    Manager, WebviewWindow, Emitter, menu::{Menu, MenuItem}, tray::{TrayIconBuilder, TrayIconEvent}, AppHandle, LogicalPosition, LogicalSize, Position,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

// Global app handle for emitting events - using OnceLock for thread safety
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

mod audio;
mod config;
mod history;
mod hotkey;
mod transcription;
mod typing;

use config::Config;

#[cfg(target_os = "linux")]
async fn check_request_audio_portal(app_handle: &tauri::AppHandle) -> Result<(), String> {
    use ashpd::desktop::camera::Camera;
    
    println!("Checking Audio/Microphone portal status (via Camera portal)...");
    
    match Camera::new().await {
        Ok(proxy) => {
            match proxy.request_access().await {
                Ok(_request) => {
                    println!("✅ Audio/Camera portal request sent");
                    Ok(())
                },
                Err(e) => {
                    let error_msg = format!("{}", e);
                    println!("⚠️ Audio/Camera portal request failed: {}", error_msg);
                    if !error_msg.contains("not found") {
                        let _ = app_handle.emit("audio-error", "portal-denied");
                    }
                    Ok(()) 
                }
            }
        },
        Err(e) => {
            println!("⚠️ Audio/Camera portal not available ({}). PulseAudio policy will manage access.", e);
            Ok(())
        }
    }
}

// Application state
#[derive(Default)]
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub is_recording: Arc<Mutex<bool>>,
    pub overlay_window: Arc<Mutex<Option<WebviewWindow>>>,
    pub hotkey_error: Arc<Mutex<Option<String>>>,
    pub setup_status: Arc<Mutex<Option<String>>>,
    pub hotkey_watch_codes: Arc<Mutex<Vec<u16>>>,
}

#[cfg(target_os = "linux")]
async fn check_and_request_permissions(app_handle: &tauri::AppHandle) -> Result<(), String> {
    use std::process::Command;

    println!("Checking system portals and groups...");

    // 1. Check groups (Silent detection)
    let groups_output = match Command::new("groups").output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(e) => {
            println!("⚠️ Failed to run 'groups' command: {}", e);
            return Ok(()); // Continue anyway
        }
    };
    
    let is_in_audio = groups_output.contains("audio");
    let is_in_input = groups_output.contains("input");

    if !is_in_audio || !is_in_input {
        println!("🔧 Missing group memberships. Triggering standard Polkit request...");
        let _ = app_handle.emit("setup-status", "configuring-system");
        
        let username = std::env::var("USER").unwrap_or_default();
        if username.is_empty() {
             println!("⚠️ Could not determine username, skipping group update.");
             return Ok(());
        }

        let cmd = format!("usermod -aG audio,input {}", username);
        
        let output = Command::new("pkexec")
            .args(&["bash", "-c", &cmd])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                println!("✅ Groups updated. Notifying user to restart session.");
                let _ = app_handle.emit("setup-status", "restart-required");
            },
            _ => {
                println!("⚠️ Group update failed or cancelled. Proceeding with existing permissions.");
                let _ = app_handle.emit("setup-status", "setup-failed");
            }
        }
    }

    Ok(())
}

// Tauri commands
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
    
    println!("Manual hotkey registration requested: {}", hotkey_string);
    
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
    println!("📡 Tauri Command: get_audio_devices invoked");
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
    println!("🎤 start_recording command - Flag set to true immediately");

    let is_recording_clone = state.is_recording.clone();
    let config = state.config.clone();
    let overlay_window = state.overlay_window.clone();
    let app_handle_clone = app_handle.clone();

    // Start recording in background
    tokio::spawn(async move {
        emit_status_update("Recording").await;
        let result = record_and_transcribe(config, is_recording_clone, overlay_window, app_handle_clone).await;
        
        if let Err(e) = result {
            log::error!("Recording/transcription error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
async fn stop_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut recording = state.is_recording.lock().unwrap();
    *recording = false;
    Ok(())
}

#[tauri::command]
async fn get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().unwrap();
    Ok(config.clone())
}

#[tauri::command]
async fn save_config(
    state: tauri::State<'_, AppState>,
    new_config: Config,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    println!("save_config command called with: {:?}", new_config);
    
    {
        let mut config = state.config.lock().unwrap();
        *config = new_config.clone();
        println!("Updated in-memory config");
        
        // Update Linux watch codes
        #[cfg(target_os = "linux")]
        {
            let mut watch_codes = state.hotkey_watch_codes.lock().unwrap();
            *watch_codes = hotkey::get_linux_key_codes(&new_config.hotkey);
            println!("🔧 Updated hotkey watch codes: {:?}", *watch_codes);
        }
    }
    
    // Save config to file first
    if let Err(e) = config::save_config(&new_config) {
        let error_msg = format!("Failed to save config: {}", e);
        println!("{}", error_msg);
        return Err(error_msg);
    }
    
    println!("Config saved successfully");
    
    // Re-register the hotkey with the new configuration
    if let Err(e) = re_register_hotkey(&app_handle, &new_config.hotkey).await {
        println!("Failed to re-register hotkey: {}", e);
        let mut error_lock = state.hotkey_error.lock().unwrap();
        *error_lock = Some(e.clone());
        return Err(format!("Config saved but failed to update hotkey: {}", e));
    } else {
        let mut error_lock = state.hotkey_error.lock().unwrap();
        *error_lock = None;
    }
    
    Ok(())
}

async fn re_register_hotkey(app_handle: &tauri::AppHandle, hotkey_string: &str) -> Result<(), String> {
    println!("Re-registering hotkey: {}", hotkey_string);
    
    // First, unregister all existing shortcuts
    if let Err(e) = app_handle.global_shortcut().unregister_all() {
        println!("Warning: Failed to unregister existing shortcuts: {}", e);
    }
    
    // Parse and register the new hotkey
    match hotkey::parse_hotkey_string(hotkey_string) {
        Ok(shortcut) => {
            match app_handle.global_shortcut().register(shortcut) {
                Ok(()) => {
                    println!("✅ Hotkey re-registered successfully: {}", hotkey_string);
                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("Failed to register new hotkey: {}", e);
                    println!("❌ {}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to parse hotkey string '{}': {}", hotkey_string, e);
            println!("❌ {}", error_msg);
            Err(error_msg)
        }
    }
}

#[tauri::command]
async fn test_api_key(api_key: String, api_url: String) -> Result<bool, String> {
    transcription::test_api_key(&api_key, &api_url).await.map_err(|e| e.to_string())
}

// Global status for overlay - using OnceLock for thread safety
static CURRENT_STATUS: OnceLock<Mutex<String>> = OnceLock::new();

#[tauri::command]
async fn get_current_status() -> Result<String, String> {
    if let Some(status_mutex) = CURRENT_STATUS.get() {
        if let Ok(status) = status_mutex.lock() {
            Ok(status.clone())
        } else {
            Ok("Ready".to_string())
        }
    } else {
        Ok("Ready".to_string())
    }
}

#[tauri::command]
async fn get_history() -> Result<history::History, String> {
    history::load_history().map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_history() -> Result<(), String> {
    history::clear_history().map_err(|e| e.to_string())
}

fn update_global_status(status: &str) {
    if let Some(status_mutex) = CURRENT_STATUS.get() {
        if let Ok(mut global_status) = status_mutex.lock() {
            *global_status = status.to_string();
        }
    }
}

async fn show_overlay_window(app_handle: &AppHandle) -> Result<(), String> {
    // Get or create the overlay window
    let overlay_window = if let Some(window) = app_handle.get_webview_window("overlay") {
        window
    } else {
        return Err("Overlay window not found".to_string());
    };

    // Position the overlay window at bottom center
    position_overlay_window(&overlay_window, app_handle).await?;
    
    // Show the overlay window
    overlay_window.show().map_err(|e| e.to_string())?;
    
    // Wayland fix: Reposition shortly after showing to bypass compositor restrictions
    let overlay_window_clone = overlay_window.clone();
    let app_handle_clone = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        let _ = position_overlay_window(&overlay_window_clone, &app_handle_clone).await;
    });
    
    Ok(())
}

async fn hide_overlay_window(app_handle: &AppHandle) -> Result<(), String> {
    if let Some(overlay_window) = app_handle.get_webview_window("overlay") {
        overlay_window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn position_overlay_window(overlay_window: &WebviewWindow, app_handle: &AppHandle) -> Result<(), String> {
    // Get the primary monitor
    let primary_monitor = overlay_window.primary_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| {
            overlay_window.available_monitors()
                .ok()
                .and_then(|monitors| monitors.first().cloned())
        })
        .ok_or("No monitors found")?;
    
    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();
    
    // Get config for pixels from bottom
    let app_state = app_handle.state::<AppState>();
    let pixels_from_bottom = {
        let config = app_state.config.lock().unwrap();
        config.pixels_from_bottom as i32
    };
    
    // Calculate position (bottom center)
    let window_width = 140;
    let window_height = 140;
    
    let x = monitor_position.x + (monitor_size.width as i32 - window_width) / 2;
    let y = monitor_position.y + monitor_size.height as i32 - window_height - pixels_from_bottom;
    
    // Set window position
    overlay_window
        .set_position(Position::Logical(LogicalPosition::new(x as f64, y as f64)))
        .map_err(|e| e.to_string())?;
    
    // Ensure window size is correct
    overlay_window
        .set_size(LogicalSize::new(window_width as f64, window_height as f64))
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

// Centralized status emitter
async fn emit_status_update(status: &str) {
    update_global_status(status);
    
    if let Some(app_handle) = APP_HANDLE.get() {
        let windows = ["main", "overlay"];
        for window_label in &windows {
            if let Some(window) = app_handle.get_webview_window(window_label) {
                if let Err(e) = window.emit("status-update", status) {
                    eprintln!("Failed to emit status to {}: {}", window_label, e);
                }
            }
        }
        
        if status == "Ready" {
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
    if audio_data.len() < 44 {
        return Err("Audio file too small to contain valid WAV header".into());
    }
    
    let sample_rate = u32::from_le_bytes([
        audio_data[24], audio_data[25], audio_data[26], audio_data[27]
    ]);
    let channels = u16::from_le_bytes([audio_data[22], audio_data[23]]);
    let bits_per_sample = u16::from_le_bytes([audio_data[34], audio_data[35]]);
    
    let mut data_size = 0u32;
    let mut pos = 36;
    
    while pos + 8 <= audio_data.len() {
        let chunk_id = &audio_data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            audio_data[pos + 4], audio_data[pos + 5], 
            audio_data[pos + 6], audio_data[pos + 7]
        ]);
        
        if chunk_id == b"data" {
            data_size = chunk_size;
            break;
        }
        
        pos += 8 + chunk_size as usize;
        if chunk_size % 2 == 1 {
            pos += 1;
        }
    }
    
    if data_size == 0 {
        return Err("No data chunk found in WAV file".into());
    }
    
    let bytes_per_sample = (bits_per_sample / 8) as u32;
    let bytes_per_second = sample_rate * channels as u32 * bytes_per_sample;
    let duration_seconds = data_size as f64 / bytes_per_second as f64;
    
    println!("Audio duration: {:.3} seconds (sample rate: {}Hz, channels: {}, bits: {})", 
             duration_seconds, sample_rate, channels, bits_per_sample);
    
    const MIN_DURATION: f64 = 0.1;
    if duration_seconds < MIN_DURATION {
        return Err(format!(
            "Audio duration {:.3}s is below minimum required {:.1}s for transcription", 
            duration_seconds, MIN_DURATION
        ).into());
    }
    
    Ok(())
}

async fn record_and_transcribe(
    config: Arc<Mutex<Config>>,
    is_recording: Arc<Mutex<bool>>,
    _overlay_window: Arc<Mutex<Option<WebviewWindow>>>,
    app_handle: AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting audio recording...");
    
    let reset_status_on_exit = || async {
        emit_status_to_frontend("Ready").await;
    };
    
    let preferred_device = {
        let config = config.lock().unwrap();
        let device = config.audio_device.clone();
        println!("🔧 Configured recording device: {}", device.as_deref().unwrap_or("System Default (Pulse)"));
        device
    };

    let audio_data = match audio::record_audio_while_flag(&is_recording, preferred_device).await {
        Ok(data) => data,
        Err(e) => {
            let error_msg = e.to_string();
            log::error!("Audio recording failed: {}", error_msg);
            
            if error_msg.contains("dsnoop") || error_msg.contains("Busy") || error_msg.contains("no longer available") {
                let _ = app_handle.emit("audio-error", "device-busy");
            } else {
                let _ = app_handle.emit("audio-error", format!("failed:{}", error_msg));
            }

            reset_status_on_exit().await;
            return Err(e);
        }
    };
    
    if audio_data.is_empty() {
        println!("No audio data recorded");
        reset_status_on_exit().await;
        return Ok(());
    }
    
    if let Err(e) = validate_audio_duration(&audio_data) {
        println!("Audio validation failed: {}", e);
        reset_status_on_exit().await;
        return Ok(());
    }
    
    emit_status_to_frontend("Converting audio").await;
    println!("Audio recorded, starting transcription...");
    
    emit_status_to_frontend("Transcribing").await;
    
    let (api_key, api_url) = {
        let config = config.lock().unwrap();
        (config.openai_api_key.clone(), config.api_url.clone())
    };
    
    if api_key.is_empty() || api_key == "your_api_key_here" {
        log::error!("API key not configured");
        reset_status_on_exit().await;
        return Err("API key not configured".into());
    }
    
    let text = match transcription::transcribe_audio(&audio_data, &api_key, &api_url).await {
        Ok(text) => text,
        Err(e) => {
            log::error!("Transcription failed: {}", e);
            let error_str = e.to_string();
            if error_str.contains("audio_too_short") || error_str.contains("Audio file is too short") {
                println!("Audio file too short for transcription (< 0.1 seconds)");
            } else {
                println!("Transcription error: {}", e);
            }
            reset_status_on_exit().await;
            return Err(e);
        }
    };
    
    if !text.trim().is_empty() {
        println!("Transcription complete, typing text: {}", text);
        
        if let Err(e) = history::add_history_item(&text) {
            println!("Warning: Failed to save to history: {}", e);
        } else {
            println!("✅ Saved transcription to history");
            if let Some(app_handle) = APP_HANDLE.get() {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.emit("history-updated", ());
                }
            }
        }
        
        emit_status_to_frontend("Typing").await;
        
        let typing_speed = {
            let config = config.lock().unwrap();
            config.typing_speed_interval
        };
        
        if let Err(e) = typing::type_text_with_config(&text, typing_speed) {
            log::error!("Typing failed: {}", e);
            reset_status_on_exit().await;
            return Err(e);
        }
        
        reset_status_on_exit().await;
    } else {
        println!("No text transcribed");
        reset_status_on_exit().await;
    }
    
    Ok(())
}

fn create_tray_menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    Menu::with_items(app, &[&open, &quit])
}

#[cfg(target_os = "linux")]
fn start_linux_input_observer(app_state: Arc<AppState>) {
    use std::fs;
    use std::time::Duration;
    
    let is_recording = app_state.is_recording.clone();
    let watch_codes = {
        let codes = app_state.hotkey_watch_codes.lock().unwrap();
        codes.clone()
    };

    if watch_codes.is_empty() {
        return;
    }

    std::thread::spawn(move || {
        println!("🚀 Linux Input Observer started. Watching for raw key codes: {:?}", watch_codes);
        
        let mut devices = Vec::new();
        if let Ok(entries) = fs::read_dir("/dev/input") {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("event") {
                        if let Ok(device) = evdev::Device::open(&path) {
                            let has_keys = device.supported_keys().map(|k| k.iter().count() > 20).unwrap_or(false);
                            if has_keys {
                                devices.push(device);
                            }
                        }
                    }
                }
            }
        }
        
        if devices.is_empty() {
            println!("⚠️ No keyboard devices found in /dev/input! Release detection will rely on OS events.");
            return;
        }

        println!("🔍 Monitoring {} hardware input devices for release events", devices.len());

        loop {
            {
                if !*is_recording.lock().unwrap() {
                    break;
                }
            }

            for device in &mut devices {
                match device.fetch_events() {
                    Ok(events) => {
                        for event in events {
                            if event.event_type() == evdev::EventType::KEY {
                                let code = event.code();
                                let value = event.value();
                                
                                // OMNI-LOGGING: Log every key event from physical hardware
                                if value == 1 {
                                    println!("⌨️  HARDWARE: Key {} PRESSED", code);
                                } else if value == 0 {
                                    println!("⌨️  HARDWARE: Key {} RELEASED", code);
                                    
                                    // ANY-KEY STOP: If any key in the combo is released, stop recording
                                    if watch_codes.contains(&code) {
                                        println!("⏹️  Combo integrity broken (Key {} released). Finalizing recording.", code);
                                        let mut recording = is_recording.lock().unwrap();
                                        *recording = false;
                                        return;
                                    }
                                }
                            }
                        }
                    },
                    Err(_) => continue,
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        println!("🏁 Linux Input Observer stopped");
    });
}

fn main() {
    env_logger::init();

    #[cfg(target_os = "linux")]
    {
        let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok() || 
                        std::env::var("XDG_SESSION_TYPE").map(|s| s == "wayland").unwrap_or(false);
        let is_x11 = std::env::var("DISPLAY").is_ok() || 
                     std::env::var("XDG_SESSION_TYPE").map(|s| s == "x11").unwrap_or(false);
        
        println!("🖥️  Display server detection: Wayland={}, X11={}", is_wayland, is_x11);
        
        if is_wayland {
            println!("🌊 Wayland detected - prioritizing Wayland backend");
            std::env::set_var("GDK_BACKEND", "wayland,x11");
            unsafe {
                if let Ok(lib) = libloading::Library::new("libgtk-3.so.0") {
                    if let Ok(gtk_init_check) = lib.get::<unsafe extern "C" fn(*mut i32, *mut *mut *mut i8) -> i32>(b"gtk_init_check") {
                        let mut argc = 0i32;
                        let mut argv = std::ptr::null_mut();
                        if gtk_init_check(&mut argc, &mut argv) != 0 {
                            println!("✅ GTK initialized for Wayland");
                        }
                    }
                }
            }
        } else if is_x11 {
            println!("🪟 X11 detected - initializing X11 threading");
            std::env::set_var("GDK_BACKEND", "x11");
            unsafe {
                if let Ok(lib) = libloading::Library::new("libX11.so.6") {
                    if let Ok(xinit_threads) = lib.get::<unsafe extern "C" fn() -> i32>(b"XInitThreads") {
                        if xinit_threads() != 0 {
                            println!("✅ X11 threading initialized successfully");
                        }
                    }
                }
                if let Ok(lib) = libloading::Library::new("libgtk-3.so.0") {
                    if let Ok(gtk_init_check) = lib.get::<unsafe extern "C" fn(*mut i32, *mut *mut *mut i8) -> i32>(b"gtk_init_check") {
                        let mut argc = 0i32;
                        let mut argv = std::ptr::null_mut();
                        if gtk_init_check(&mut argc, &mut argv) != 0 {
                            println!("✅ GTK initialized for X11");
                        }
                    }
                }
            }
        }
    }

    let is_first_launch = config::is_first_launch().unwrap_or(false);
    
    let app_state = AppState {
        config: Arc::new(Mutex::new(config::load_config().unwrap_or_default())),
        is_recording: Arc::new(Mutex::new(false)),
        overlay_window: Arc::new(Mutex::new(None)),
        hotkey_error: Arc::new(Mutex::new(None)),
        setup_status: Arc::new(Mutex::new(None)),
        hotkey_watch_codes: Arc::new(Mutex::new(Vec::new())),
    };
    
    {
        let config = app_state.config.lock().unwrap();
        let mut watch_codes = app_state.hotkey_watch_codes.lock().unwrap();
        #[cfg(target_os = "linux")]
        {
            *watch_codes = hotkey::get_linux_key_codes(&config.hotkey);
            println!("🔧 Initial hotkey watch codes: {:?}", *watch_codes);
        }
    }

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app_handle, shortcut, event| {
                    let app_state = app_handle.state::<AppState>();
                    let app_state_arc = Arc::new(AppState {
                        config: app_state.config.clone(),
                        is_recording: app_state.is_recording.clone(),
                        overlay_window: app_state.overlay_window.clone(),
                        hotkey_error: app_state.hotkey_error.clone(),
                        setup_status: app_state.setup_status.clone(),
                        hotkey_watch_codes: app_state.hotkey_watch_codes.clone(),
                    });
                    
                    println!("🔥 Shortcut Identity Event: {:?}, State: {:?}", shortcut, event.state);
                    
                    match event.state {
                        tauri_plugin_global_shortcut::ShortcutState::Pressed => {
                            let mut recording_flag = app_state.is_recording.lock().unwrap();
                            if !*recording_flag {
                                *recording_flag = true;
                                println!("🎤 Hotkey PRESSED - Flag set to true immediately");
                                
                                #[cfg(target_os = "linux")]
                                start_linux_input_observer(app_state_arc.clone());

                                let app_handle_clone = app_handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    let state = app_handle_clone.state::<AppState>();
                                    let config = state.config.clone();
                                    let overlay_window = state.overlay_window.clone();
                                    let is_recording = state.is_recording.clone();
                                    
                                    emit_status_update("Recording").await;
                                    let result = record_and_transcribe(config, is_recording, overlay_window, app_handle_clone).await;
                                    if let Err(e) = result {
                                        log::error!("Recording/transcription error: {}", e);
                                    }
                                });
                                
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    let _ = window.emit("hotkey-pressed", ());
                                }
                            }
                        }
                        tauri_plugin_global_shortcut::ShortcutState::Released => {
                            let mut recording_flag = app_state.is_recording.lock().unwrap();
                            if *recording_flag {
                                *recording_flag = false;
                                println!("⏹️  Shortcut Identity Event: RELEASED - Flag set to false immediately");
                                
                                let app_handle_emit = app_handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                                    if let Some(window) = app_handle_emit.get_webview_window("main") {
                                        let _ = window.emit("hotkey-released", ());
                                    }
                                });
                            }
                        }
                    }
                })
                .build()
        )
        .manage(app_state)
        .setup(move |app| {
            let _ = APP_HANDLE.set(app.handle().clone());
            let _ = CURRENT_STATUS.set(Mutex::new("Ready".to_string()));
            
            if let Some(overlay_window) = app.get_webview_window("overlay") {
                println!("✅ Using overlay window from config");
                let _ = overlay_window.hide();
            }
            
            let _ = audio::get_input_devices();
            let menu = create_tray_menu(app.handle())?;
            let _tray = TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app_handle, event| match event.id.as_ref() {
                    "quit" => { std::process::exit(0); }
                    "open" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click { button, .. } = event {
                        if let tauri::tray::MouseButton::Left = button {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            if is_first_launch {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            let app_handle_setup = app.handle().clone();
            #[cfg(target_os = "linux")]
            tauri::async_runtime::spawn(async move {
                let _ = check_request_audio_portal(&app_handle_setup).await;
                let _ = check_and_request_permissions(&app_handle_setup).await;
            });
            
            let hotkey_string = {
                let state = app.state::<AppState>();
                let config = state.config.lock().unwrap();
                config.hotkey.clone()
            };
            
            match hotkey::parse_hotkey_string(&hotkey_string) {
                Ok(shortcut) => {
                    let _ = app.handle().global_shortcut().register(shortcut);
                    println!("✅ Global hotkey registered: {}", hotkey_string);
                }
                Err(e) => {
                    eprintln!("❌ Failed to parse hotkey: {}", e);
                }
            }
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            get_config,
            save_config,
            test_api_key,
            get_current_status,
            get_history,
            clear_history,
            check_hotkey_status,
            manual_register_hotkey,
            get_audio_devices
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
