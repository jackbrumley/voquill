// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex, OnceLock};
use std::collections::HashSet;
use tauri::{
    Manager, WebviewWindow, Emitter, menu::{Menu, MenuItem}, tray::{TrayIconBuilder, TrayIconEvent}, AppHandle, LogicalPosition, LogicalSize, Position,
};

// Global app handle for emitting events - using OnceLock for thread safety
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

mod audio;
mod config;
mod history;
mod hotkey;
mod transcription;
mod typing;

use config::Config;
use hotkey::HardwareHotkey;

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
pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub is_recording: Arc<Mutex<bool>>,
    pub hotkey_error: Arc<Mutex<Option<String>>>,
    pub setup_status: Arc<Mutex<Option<String>>>,
    pub hardware_hotkey: Arc<Mutex<HardwareHotkey>>,
    pub cached_device: Arc<Mutex<Option<cpal::Device>>>,
    pub virtual_keyboard: Arc<Mutex<Option<evdev::uinput::VirtualDevice>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Arc::new(Mutex::new(Config::default())),
            is_recording: Arc::new(Mutex::new(false)),
            hotkey_error: Arc::new(Mutex::new(None)),
            setup_status: Arc::new(Mutex::new(None)),
            hardware_hotkey: Arc::new(Mutex::new(HardwareHotkey::default())),
            cached_device: Arc::new(Mutex::new(None)),
            virtual_keyboard: Arc::new(Mutex::new(None)),
        }
    }
}

#[cfg(target_os = "linux")]
async fn check_and_request_permissions(app_handle: &tauri::AppHandle) -> Result<(), String> {
    use std::process::Command;
    use std::fs;

    println!("Checking system portals and groups...");

    // Check uinput access directly
    let has_uinput_access = fs::OpenOptions::new().write(true).open("/dev/uinput").is_ok();
    println!("📂 /dev/uinput access: {}", if has_uinput_access { "Writable ✅" } else { "DENIED ❌" });
    
    // 2. Check group memberships
    let groups_output = match Command::new("groups").output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).into_owned(),
        Err(e) => {
            println!("⚠️ Failed to run 'groups' command: {}", e);
            return Ok(());
        }
    };
    println!("👤 User groups: {}", groups_output.trim());
    
    let is_in_audio = groups_output.contains("audio");
    let is_in_input = groups_output.contains("input");
    let is_in_uinput = groups_output.contains("uinput");

    if !has_uinput_access || !is_in_audio || !is_in_input {
        println!("🔧 Missing permissions or group memberships. Triggering Polkit request...");
        let _ = app_handle.emit("setup-status", "configuring-system");
        
        let username = std::env::var("USER").unwrap_or_default();
        if username.is_empty() {
             println!("⚠️ Could not determine username, skipping setup.");
             return Ok(());
        }

        // Determine correct group for uinput
        let uinput_group = if is_in_uinput || groups_output.contains("uinput") { "uinput" } else { "input" };
        
        let cmd = format!("usermod -aG audio,input,{} {}", uinput_group, username);
        println!("🔧 Executing: pkexec {}", cmd);
        
        let output = Command::new("pkexec")
            .args(&["bash", "-c", &cmd])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                println!("✅ Permissions updated. Notifying user to restart session.");
                let _ = app_handle.emit("setup-status", "restart-required");
            },
            _ => {
                println!("⚠️ Permission update failed or cancelled.");
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
    let app_handle_clone = app_handle.clone();
    let cached_device = state.cached_device.clone();
    let virtual_keyboard = state.virtual_keyboard.clone();

    tokio::spawn(async move {
        emit_status_update("Recording").await;
        let result = record_and_transcribe(config, is_recording_clone, app_handle_clone, cached_device, virtual_keyboard).await;
        
        if let Err(e) = result {
            println!("❌ Global Recording error: {}", e);
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
        
        // Update Hardware Hotkey for Linux
        #[cfg(target_os = "linux")]
        {
            let mut hardware_hotkey = state.hardware_hotkey.lock().unwrap();
            *hardware_hotkey = hotkey::parse_hardware_hotkey(&new_config.hotkey);
            println!("🔧 Updated hardware hotkey: {:?}", *hardware_hotkey);
        }

        // Pre-warm the audio device cache
        let mut cached_device = state.cached_device.lock().unwrap();
        *cached_device = audio::lookup_device(new_config.audio_device.clone()).ok();
        println!("🔧 Pre-warmed audio device cache");
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
        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        println!("Re-registering hotkey: {}", _hotkey_string);
        let _ = _app_handle.global_shortcut().unregister_all();
        match hotkey::parse_hotkey_string(_hotkey_string) {
            Ok(shortcut) => {
                _app_handle.global_shortcut().register(shortcut).map_err(|e| e.to_string())?;
                println!("✅ Global hotkey registered: {}", _hotkey_string);
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

async fn show_overlay_window(app_handle: &AppHandle) -> Result<(), String> {
    let overlay_window = app_handle.get_webview_window("overlay").ok_or("Overlay window not found")?;
    
    // Check if already visible to avoid redundant GTK calls that can cause panics in tao
    if overlay_window.is_visible().unwrap_or(false) {
        return Ok(());
    }

    position_overlay_window(&overlay_window, app_handle).await?;
    
    // CRITICAL: Show the window FIRST, then set click-through and non-focusable.
    // On Wayland/Linux, setting these before the window is realized can cause event loop panics.
    overlay_window.show().map_err(|e| e.to_string())?;
    
    // Explicitly ensure the window doesn't take focus and remains click-through
    let _ = overlay_window.set_focusable(false);
    let _ = overlay_window.set_ignore_cursor_events(true);
    
    println!("👻 Overlay realized and set to Ghost Mode (Non-focusable + Click-through)");
    
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
    let primary_monitor = overlay_window.primary_monitor()
        .map_err(|e| e.to_string())?
        .or_else(|| overlay_window.available_monitors().ok().and_then(|m| m.first().cloned()))
        .ok_or("No monitors found")?;
    
    let monitor_size = primary_monitor.size();
    let monitor_position = primary_monitor.position();
    
    let app_state = app_handle.state::<AppState>();
    let pixels_from_bottom = {
        let config = app_state.config.lock().unwrap();
        config.pixels_from_bottom as i32
    };
    
    let window_width = 140;
    let window_height = 140;
    let x = monitor_position.x + (monitor_size.width as i32 - window_width) / 2;
    let y = monitor_position.y + monitor_size.height as i32 - window_height - pixels_from_bottom;
    
    overlay_window.set_position(Position::Logical(LogicalPosition::new(x as f64, y as f64))).map_err(|e| e.to_string())?;
    overlay_window.set_size(LogicalSize::new(window_width as f64, window_height as f64)).map_err(|e| e.to_string())?;
    
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
    
    // If status hasn't changed, don't trigger redundant window operations or events
    if !changed {
        return;
    }
    
    if let Some(app_handle) = APP_HANDLE.get() {
        let windows = ["main", "overlay"];
        for window_label in &windows {
            if let Some(window) = app_handle.get_webview_window(window_label) {
                let _ = window.emit("status-update", status);
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
    
    println!("Audio duration: {:.3}s", duration_seconds);
    if duration_seconds < 0.1 { return Err("Audio too short".into()); }
    Ok(())
}

async fn record_and_transcribe(
    config: Arc<Mutex<Config>>,
    is_recording: Arc<Mutex<bool>>,
    app_handle: AppHandle,
    cached_device: Arc<Mutex<Option<cpal::Device>>>,
    virtual_keyboard: Arc<Mutex<Option<evdev::uinput::VirtualDevice>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reset_status_on_exit = || async { emit_status_to_frontend("Ready").await; };
    
    let audio_data = match audio::record_audio_while_flag(&is_recording, cached_device).await {
        Ok(data) => data,
        Err(e) => {
            reset_status_on_exit().await;
            return Err(e);
        }
    };
    
    if audio_data.is_empty() { reset_status_on_exit().await; return Ok(()); }
    if let Err(e) = validate_audio_duration(&audio_data) { 
        println!("⚠️ Audio validation failed: {}", e);
        reset_status_on_exit().await; 
        return Ok(()); 
    }
    
    emit_status_to_frontend("Transcribing").await;
    let (api_key, api_url) = {
        let config = config.lock().unwrap();
        (config.openai_api_key.clone(), config.api_url.clone())
    };
    
    println!("📡 Sending {} bytes to transcription API...", audio_data.len());
    let text = match transcription::transcribe_audio(&audio_data, &api_key, &api_url).await {
        Ok(text) => {
            println!("📝 Transcription received: \"{}\"", text);
            text
        },
        Err(e) => { 
            println!("❌ Transcription API failed: {}", e);
            reset_status_on_exit().await; 
            return Err(e); 
        }
    };
    
    if !text.trim().is_empty() {
        let _ = history::add_history_item(&text);
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.emit("history-updated", ());
        }
        
        emit_status_to_frontend("Typing").await;
        let typing_speed = { config.lock().unwrap().typing_speed_interval };
        
        // Give the OS a moment to ensure focus is on the target app
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        
        println!("⌨️  Forwarding text to hardware typing engine...");
        if let Err(e) = typing::type_text_hardware(&text, typing_speed, virtual_keyboard) {
            println!("❌ TYPING ENGINE ERROR: {}", e);
        }
    } else {
        println!("ℹ️ Transcription was empty, skipping typing.");
    }
    
    reset_status_on_exit().await;
    Ok(())
}

fn create_tray_menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    Menu::with_items(app, &[&open, &quit])
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
        println!("🚀 Linux Hardware Input Engine started.");
        
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
                                println!("🔍 Monitoring hardware: {} ({})", dev_name, path.display());
                                devices.push(device);
                            }
                        }
                    }
                }
            }
        }
        
        if devices.is_empty() {
            println!("⚠️ No keyboards found! Input engine disabled.");
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
                                if event.event_type() == evdev::EventType::KEY {
                                    let code = event.code();
                                    let value = event.value();
                                    let h_hotkey = hardware_hotkey_flag.lock().unwrap().clone();

                                    if value == 1 { // Pressed
                                        println!("⌨️  [{}] Key {} PRESSED", dev_name, code);
                                        pressed_keys.insert(code);
                                        
                                        // CHECK FOR START: All combo keys must be in pressed_keys
                                        let all_pressed = !h_hotkey.all_codes.is_empty() && 
                                                         h_hotkey.all_codes.iter().all(|c| pressed_keys.contains(c));
                                        
                                        if all_pressed {
                                            let mut recording = is_recording_flag.lock().unwrap();
                                            if !*recording {
                                                *recording = true;
                                                println!("🎤 ENGINE: Combination Met! Starting recording.");
                                                
                                                let h_clone = app_handle.clone();
                                                tauri::async_runtime::spawn(async move {
                                                    emit_status_update("Recording").await;
                                                    let s = h_clone.state::<AppState>();
                                                    let config = s.config.clone();
                                                    let is_recording = s.is_recording.clone();
                                                    let cached_device = s.cached_device.clone();
                                                    let virtual_keyboard = s.virtual_keyboard.clone();
                                                    
                                                    let _ = record_and_transcribe(config, is_recording, h_clone, cached_device, virtual_keyboard).await;
                                                });
                                                
                                                if let Some(w) = app_handle.get_webview_window("main") {
                                                    let _ = w.emit("hotkey-pressed", ());
                                                }
                                            }
                                        }
                                    } else if value == 0 { // Released
                                        println!("⌨️  [{}] Key {} RELEASED", dev_name, code);
                                        pressed_keys.remove(&code);
                                        
                                        // CHECK FOR STOP: If we ARE recording and ANY combo key is released
                                        let is_combo_key = h_hotkey.all_codes.contains(&code);
                                        let mut recording = is_recording_flag.lock().unwrap();
                                        if *recording && is_combo_key {
                                            *recording = false;
                                            println!("⏹️  ENGINE: Key Released! Finalizing.");
                                            
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
    env_logger::init();

    let is_first_launch = config::is_first_launch().unwrap_or(false);
    let initial_config = config::load_config().unwrap_or_default();
    
    let app_state = AppState {
        config: Arc::new(Mutex::new(initial_config.clone())),
        hardware_hotkey: Arc::new(Mutex::new(hotkey::parse_hardware_hotkey(&initial_config.hotkey))),
        ..Default::default()
    };

    {
        let mut cached_device = app_state.cached_device.lock().unwrap();
        *cached_device = audio::lookup_device(initial_config.audio_device.clone()).ok();
        println!("🔧 Initial pre-warm of audio device cache complete");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state)
        .setup(move |app| {
            let _ = APP_HANDLE.set(app.handle().clone());
            let _ = CURRENT_STATUS.set(Mutex::new("Ready".to_string()));
            
            if let Some(w) = app.get_webview_window("overlay") { let _ = w.hide(); }
            let _ = audio::get_input_devices();
            
            #[cfg(target_os = "linux")]
            start_linux_input_engine(app.handle().clone());

            // Hardware-Level Virtual Keyboard Initialization
            #[cfg(target_os = "linux")]
            {
                let state = app.state::<AppState>();
                let virtual_keyboard = state.virtual_keyboard.clone();
                std::thread::spawn(move || {
                    use evdev::uinput::VirtualDeviceBuilder;
                    use evdev::{AttributeSet, Key, InputId, BusType};
                    println!("🔄 Starting virtual hardware keyboard initialization...");
                    
                    let mut keys = AttributeSet::<Key>::new();
                    // Add standard alphanumeric keys and symbols
                    for i in 0..564 {
                        keys.insert(Key::new(i as u16));
                    }

                    let input_id = InputId::new(BusType::BUS_USB, 0x6666, 0x8888, 0x0111);

                    match VirtualDeviceBuilder::new()
                        .unwrap()
                        .name("Voquill Virtual Keyboard")
                        .input_id(input_id)
                        .with_keys(&keys)
                        .unwrap()
                        .build() 
                    {
                        Ok(mut device) => {
                            if let Ok(path) = device.get_syspath() {
                                println!("✅ Virtual hardware keyboard initialized at: {}", path.display());
                            } else {
                                println!("✅ Virtual hardware keyboard initialized");
                            }
                            let mut lock = virtual_keyboard.lock().unwrap();
                            *lock = Some(device);
                        },
                        Err(e) => {
                            println!("❌ Virtual keyboard initialization failed: {}", e);
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

            if is_first_launch { if let Some(w) = app.get_webview_window("main") { let _ = w.show(); } }

            let h = app.handle().clone();
            #[cfg(target_os = "linux")]
            tauri::async_runtime::spawn(async move {
                let _ = check_request_audio_portal(&h).await;
                let _ = check_and_request_permissions(&h).await;
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording, stop_recording, get_config, save_config,
            test_api_key, get_current_status, get_history, clear_history,
            check_hotkey_status, manual_register_hotkey, get_audio_devices
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
