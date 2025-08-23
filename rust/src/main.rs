// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use tauri::{
    Manager, WebviewWindow, Emitter, menu::{Menu, MenuItem}, tray::{TrayIconBuilder, TrayIconEvent}, 
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

mod audio;
mod config;
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
    show_overlay(&window, "Recording...").await?;

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
async fn test_api_key(api_key: String) -> Result<bool, String> {
    transcription::test_api_key(&api_key).await.map_err(|e| e.to_string())
}

async fn show_overlay(main_window: &WebviewWindow, message: &str) -> Result<(), String> {
    // For now, just emit to the main window - we'll implement proper overlay later
    main_window
        .emit("status-update", message)
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn record_and_transcribe(
    config: Arc<Mutex<Config>>,
    is_recording: Arc<Mutex<bool>>,
    _overlay_window: Arc<Mutex<Option<WebviewWindow>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting audio recording...");
    
    // Record audio while the recording flag is true
    let audio_data = audio::record_audio_while_flag(&is_recording).await?;
    
    if audio_data.is_empty() {
        println!("No audio data recorded");
        return Ok(());
    }
    
    println!("Audio recorded, starting transcription...");
    
    // Transcribe audio
    let api_key = {
        let config = config.lock().unwrap();
        config.openai_api_key.clone()
    };
    
    if api_key.is_empty() || api_key == "your_api_key_here" {
        return Err("OpenAI API key not configured".into());
    }
    
    let text = transcription::transcribe_audio(&audio_data, &api_key).await?;
    
    if !text.trim().is_empty() {
        println!("Transcription complete, typing text: {}", text);
        // Type the text using config speed
        let typing_speed = {
            let config = config.lock().unwrap();
            config.typing_speed_interval
        };
        typing::type_text_with_config(&text, typing_speed)?;
    } else {
        println!("No text transcribed");
    }
    
    Ok(())
}

fn create_tray_menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let record = MenuItem::with_id(app, "record", "Start Recording", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    
    Menu::with_items(app, &[&show, &record, &quit])
}

fn main() {
    env_logger::init();

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
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit("hotkey-pressed", ());
                            }
                        }
                        tauri_plugin_global_shortcut::ShortcutState::Released => {
                            println!("⏹️ Hotkey RELEASED - Stopping recording");
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.emit("hotkey-released", ());
                            }
                        }
                    }
                })
                .build()
        )
        .manage(app_state)
        .setup(|app| {
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
                    "show" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "record" => {
                        // TODO: Trigger recording from system tray
                    }
                    _ => {}
                })
                .on_tray_icon_event(move |tray, event| {
                    if let TrayIconEvent::Click { .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
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
            test_api_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
