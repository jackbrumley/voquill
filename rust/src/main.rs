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

// Application state
#[derive(Default)]
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub is_recording: Arc<Mutex<bool>>,
    pub overlay_window: Arc<Mutex<Option<WebviewWindow>>>,
}

// Tauri commands
#[tauri::command]
async fn start_recording(
    state: tauri::State<'_, AppState>,
    window: WebviewWindow,
) -> Result<(), String> {
    let is_recording = state.is_recording.clone();
    let config = state.config.clone();
    let overlay_window = state.overlay_window.clone();

    // Check if already recording
    {
        let recording = is_recording.lock().unwrap();
        if *recording {
            return Err("Already recording".to_string());
        }
    }

    // Set recording state
    {
        let mut recording = is_recording.lock().unwrap();
        *recording = true;
    }

    // Show overlay window
    show_overlay(&window, "Recording").await?;

    // Start recording in background
    let is_recording_clone = is_recording.clone();
    tokio::spawn(async move {
        let result = record_and_transcribe(config, is_recording_clone, overlay_window).await;
        
        // Reset recording state
        {
            let mut recording = is_recording.lock().unwrap();
            *recording = false;
        }

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
        return Err(format!("Config saved but failed to update hotkey: {}", e));
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
    
    Ok(())
}

async fn hide_overlay_window(app_handle: &AppHandle) -> Result<(), String> {
    if let Some(overlay_window) = app_handle.get_webview_window("overlay") {
        overlay_window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn position_overlay_window(overlay_window: &WebviewWindow, app_handle: &AppHandle) -> Result<(), String> {
    // Get the primary monitor (the actual primary display configured by the user)
    let primary_monitor = overlay_window.primary_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| {
            // Fallback: get all monitors and use the first one if primary detection fails
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

// Centralized status emitter - single source of truth for all status updates
async fn emit_status_update(status: &str) {
    // Update global status
    update_global_status(status);
    
    if let Some(app_handle) = APP_HANDLE.get() {
        // Emit to ALL windows with the same event
        let windows = ["main", "overlay"];
        for window_label in &windows {
            if let Some(window) = app_handle.get_webview_window(window_label) {
                if let Err(e) = window.emit("status-update", status) {
                    eprintln!("Failed to emit status to {}: {}", window_label, e);
                }
            }
        }
        
        // Handle overlay visibility based on status
        if status == "Ready" {
            let _ = hide_overlay_window(app_handle).await;
        } else {
            let _ = show_overlay_window(app_handle).await;
        }
    }
}

// Legacy function for backward compatibility - now just calls the centralized emitter
async fn emit_status_to_frontend(status: &str) {
    emit_status_update(status).await;
}

// Legacy function for backward compatibility - now just calls the centralized emitter
async fn show_overlay(_main_window: &WebviewWindow, message: &str) -> Result<(), String> {
    emit_status_update(message).await;
    Ok(())
}

// Validate audio duration to ensure it meets minimum requirements for transcription
fn validate_audio_duration(audio_data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Parse WAV header to get duration information
    if audio_data.len() < 44 {
        return Err("Audio file too small to contain valid WAV header".into());
    }
    
    // Read WAV header fields
    let sample_rate = u32::from_le_bytes([
        audio_data[24], audio_data[25], audio_data[26], audio_data[27]
    ]);
    let channels = u16::from_le_bytes([audio_data[22], audio_data[23]]);
    let bits_per_sample = u16::from_le_bytes([audio_data[34], audio_data[35]]);
    
    // Find the data chunk
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
        // Align to even byte boundary
        if chunk_size % 2 == 1 {
            pos += 1;
        }
    }
    
    if data_size == 0 {
        return Err("No data chunk found in WAV file".into());
    }
    
    // Calculate duration in seconds
    let bytes_per_sample = (bits_per_sample / 8) as u32;
    let bytes_per_second = sample_rate * channels as u32 * bytes_per_sample;
    let duration_seconds = data_size as f64 / bytes_per_second as f64;
    
    println!("Audio duration: {:.3} seconds (sample rate: {}Hz, channels: {}, bits: {})", 
             duration_seconds, sample_rate, channels, bits_per_sample);
    
    // OpenAI Whisper requires minimum 0.1 seconds
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting audio recording...");
    
    // Ensure we always reset status to Ready, even if errors occur
    let reset_status_on_exit = || async {
        emit_status_to_frontend("Ready").await;
    };
    
    // Record audio while the recording flag is true
    let audio_data = match audio::record_audio_while_flag(&is_recording).await {
        Ok(data) => data,
        Err(e) => {
            log::error!("Audio recording failed: {}", e);
            reset_status_on_exit().await;
            return Err(e);
        }
    };
    
    if audio_data.is_empty() {
        println!("No audio data recorded");
        reset_status_on_exit().await;
        return Ok(());
    }
    
    // Validate audio duration before proceeding with transcription
    if let Err(e) = validate_audio_duration(&audio_data) {
        println!("Audio validation failed: {}", e);
        reset_status_on_exit().await;
        return Ok(()); // Not an error, just too short
    }
    
    // Emit status update for audio conversion (if needed)
    emit_status_to_frontend("Converting audio").await;
    println!("Audio recorded, starting transcription...");
    
    // Emit status update for transcription
    emit_status_to_frontend("Transcribing").await;
    
    // Get API key and URL
    let (api_key, api_url) = {
        let config = config.lock().unwrap();
        (config.openai_api_key.clone(), config.api_url.clone())
    };
    
    if api_key.is_empty() || api_key == "your_api_key_here" {
        log::error!("API key not configured");
        reset_status_on_exit().await;
        return Err("API key not configured".into());
    }
    
    // Transcribe audio with proper error handling
    let text = match transcription::transcribe_audio(&audio_data, &api_key, &api_url).await {
        Ok(text) => text,
        Err(e) => {
            log::error!("Transcription failed: {}", e);
            
            // Check if it's the "audio too short" error specifically
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
        
        // Save to history
        if let Err(e) = history::add_history_item(&text) {
            println!("Warning: Failed to save to history: {}", e);
        } else {
            println!("✅ Saved transcription to history");
            
            // Emit history update event to frontend
            if let Some(app_handle) = APP_HANDLE.get() {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.emit("history-updated", ());
                }
            }
        }
        
        // Emit status update for typing
        emit_status_to_frontend("Typing").await;
        
        // Type the text using config speed
        let typing_speed = {
            let config = config.lock().unwrap();
            config.typing_speed_interval
        };
        
        if let Err(e) = typing::type_text_with_config(&text, typing_speed) {
            log::error!("Typing failed: {}", e);
            reset_status_on_exit().await;
            return Err(e);
        }
        
        // Emit final status update
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

fn main() {
    env_logger::init();

    // Initialize threading for Linux GUI applications with smart Wayland/X11 detection
    #[cfg(target_os = "linux")]
    {
        // Detect the current display server
        let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok() || 
                        std::env::var("XDG_SESSION_TYPE").map(|s| s == "wayland").unwrap_or(false);
        
        let is_x11 = std::env::var("DISPLAY").is_ok() || 
                     std::env::var("XDG_SESSION_TYPE").map(|s| s == "x11").unwrap_or(false);
        
        println!("🖥️  Display server detection: Wayland={}, X11={}", is_wayland, is_x11);
        
        if is_wayland {
            println!("🌊 Wayland detected - prioritizing Wayland backend");
            std::env::set_var("GDK_BACKEND", "wayland,x11");
            
            // Initialize Wayland-compatible threading
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
            
            // Initialize X11 threading before any X11 operations
            unsafe {
                // Initialize X11 threading first
                if let Ok(lib) = libloading::Library::new("libX11.so.6") {
                    if let Ok(xinit_threads) = lib.get::<unsafe extern "C" fn() -> i32>(b"XInitThreads") {
                        let result = xinit_threads();
                        if result != 0 {
                            println!("✅ X11 threading initialized successfully");
                        } else {
                            println!("⚠️  X11 threading initialization returned 0 (may already be initialized)");
                        }
                    }
                }
                
                // Then initialize GTK
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
        } else {
            println!("❓ Unknown display server - using fallback initialization");
            std::env::set_var("GDK_BACKEND", "wayland,x11");
            
            // Fallback: try both Wayland and X11 initialization
            unsafe {
                // Try X11 threading first (safer)
                if let Ok(lib) = libloading::Library::new("libX11.so.6") {
                    if let Ok(xinit_threads) = lib.get::<unsafe extern "C" fn() -> i32>(b"XInitThreads") {
                        xinit_threads();
                        println!("✅ X11 threading initialized (fallback)");
                    }
                }
                
                // Then GTK
                if let Ok(lib) = libloading::Library::new("libgtk-3.so.0") {
                    if let Ok(gtk_init_check) = lib.get::<unsafe extern "C" fn(*mut i32, *mut *mut *mut i8) -> i32>(b"gtk_init_check") {
                        let mut argc = 0i32;
                        let mut argv = std::ptr::null_mut();
                        gtk_init_check(&mut argc, &mut argv);
                        println!("✅ GTK initialized (fallback)");
                    }
                }
            }
        }
    }

    // Check for first launch BEFORE loading config (which creates the file)
    let is_first_launch = config::is_first_launch().unwrap_or(false);
    
    let app_state = AppState {
        config: Arc::new(Mutex::new(config::load_config().unwrap_or_default())),
        is_recording: Arc::new(Mutex::new(false)),
        overlay_window: Arc::new(Mutex::new(None)),
    };

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app_handle, shortcut, event| {
                    println!("🔥 Hotkey event received! Shortcut: {:?}, State: {:?}", shortcut, event.state);
                    match event.state {
                        tauri_plugin_global_shortcut::ShortcutState::Pressed => {
                            println!("🎤 Hotkey PRESSED - Starting recording");
                            
                            // Use centralized status emitter
                            tauri::async_runtime::spawn(async move {
                                emit_status_update("Recording").await;
                            });
                            
                            // Emit hotkey event to main window
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit("hotkey-pressed", ());
                            }
                        }
                        tauri_plugin_global_shortcut::ShortcutState::Released => {
                            println!("⏹️ Hotkey RELEASED - Stopping recording");
                            
                            // Add delay to allow system to properly clear keyboard state
                            // This helps prevent "stuck spacebar" issues with wireless keyboards
                            let app_handle_clone = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                                
                                // Don't hide overlay immediately - let it stay visible during transcription/typing
                                // The overlay will be hidden when the process completes (status becomes "Ready")
                                
                                // Emit to main window after delay
                                if let Some(window) = app_handle_clone.get_webview_window("main") {
                                    let _ = window.emit("hotkey-released", ());
                                }
                            });
                        }
                    }
                })
                .build()
        )
        .manage(app_state)
        .setup(move |app| {
            // Store the app handle globally for status updates
            let _ = APP_HANDLE.set(app.handle().clone());
            
            // Initialize the global status
            let _ = CURRENT_STATUS.set(Mutex::new("Ready".to_string()));
            
            // The overlay window is already defined in tauri.conf.json with the correct URL (/overlay)
            // Just ensure it's hidden initially
            if let Some(overlay_window) = app.get_webview_window("overlay") {
                println!("✅ Using overlay window from config");
                let _ = overlay_window.hide();
            } else {
                println!("❌ Overlay window not found in config");
            }
            
            // Create tray menu
            let menu = create_tray_menu(app.handle())?;
            
            // Create tray icon
            let _tray = TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app_handle, event| match event.id.as_ref() {
                    "quit" => {
                        std::process::exit(0);
                    }
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
                        match button {
                            tauri::tray::MouseButton::Left => {
                                if let Some(window) = tray.app_handle().get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                            tauri::tray::MouseButton::Right => {
                                // Do nothing - let the context menu appear naturally
                            }
                            _ => {}
                        }
                    }
                })
                .build(app)?;

            // Handle window close event to hide instead of exit
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // Prevent the window from closing
                        api.prevent_close();
                        // Hide the window instead
                        let _ = window_clone.hide();
                    }
                });
            }

            // Check if this is first launch and show window if needed
            if is_first_launch {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                    println!("First launch detected - showing main window for configuration");
                }
            }

            // Register the global hotkey from config
            let app_handle = app.handle().clone();
            let app_state = app.state::<AppState>();
            let hotkey_string = {
                let config = app_state.config.lock().unwrap();
                config.hotkey.clone()
            };
            
            println!("Attempting to register hotkey: {}", hotkey_string);
            
            // Parse and register the hotkey
            match hotkey::parse_hotkey_string(&hotkey_string) {
                Ok(shortcut) => {
                    println!("Parsed hotkey successfully: {:?}", shortcut);
                    match app_handle.global_shortcut().register(shortcut) {
                        Ok(()) => {
                            println!("✅ Global hotkey registered successfully: {}", hotkey_string);
                            log::info!("Global hotkey registered: {}", hotkey_string);
                        }
                        Err(e) => {
                            println!("❌ Failed to register global hotkey: {}", e);
                            log::error!("Failed to register global hotkey: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to parse hotkey string '{}': {}", hotkey_string, e);
                    log::error!("Failed to parse hotkey string: {}", hotkey_string);
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
            clear_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
