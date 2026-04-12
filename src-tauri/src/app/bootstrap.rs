use crate::app::commands::hotkey::re_register_hotkey;
#[cfg(target_os = "linux")]
use crate::app::commands::platform::is_status_notifier_watcher_available;
use crate::app::state::AppState;
use crate::config::Config;
use crate::{audio, hotkey};
#[cfg(target_os = "linux")]
use ashpd::{register_host_app, AppID};
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    Manager,
};

#[cfg(target_os = "linux")]
use crate::platform::linux::detection::is_wayland_session;
#[cfg(target_os = "linux")]
use crate::platform::linux::wayland::env::check_wayland_display;

#[cfg(target_os = "linux")]
fn read_linux_distribution_name() -> Option<String> {
    let contents = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return Some(value.trim_matches('"').to_string());
        }
    }
    None
}

fn create_tray_menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let open_item = MenuItem::with_id(app, "open", "Open Voquill", true, None::<&str>)?;
    Menu::with_items(app, &[&open_item, &quit_item])
}

pub fn build_app_state(initial_config: &Config) -> AppState {
    let app_state = AppState {
        config: Arc::new(Mutex::new(initial_config.clone())),
        hardware_hotkey: Arc::new(Mutex::new(hotkey::parse_hardware_hotkey(
            &initial_config.hotkey,
        ))),
        ..Default::default()
    };

    {
        let mut cached_device = app_state.cached_device.lock().unwrap();
        let device = audio::lookup_device(initial_config.audio_device.clone()).ok();
        *cached_device = device.clone();

        if let Some(device) = device {
            if let Ok(engine) =
                audio::PersistentAudioEngine::new(&device, initial_config.input_sensitivity)
            {
                let mut engine_guard = app_state.audio_engine.lock().unwrap();
                *engine_guard = Some(engine);
                crate::log_info!("✅ Persistent audio engine initialized");
            }
        }
        crate::log_info!("🔧 Initial pre-warm of audio device cache complete");
    }

    app_state
}

pub fn run_setup(
    app: &mut tauri::App<tauri::Wry>,
    initial_config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    crate::app::status::initialize(app.handle().clone());

    #[cfg(target_os = "linux")]
    {
        check_wayland_display();
        let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        let x11_display = std::env::var("DISPLAY").ok();
        let session_type = std::env::var("XDG_SESSION_TYPE").ok();
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
        let prg_name = gtk::glib::prgname();
        let detected = crate::platform::linux::detection::detect_display_server();
        crate::log_info!(
            "🧭 Launch context: detected={:?}, XDG_SESSION_TYPE={:?}, WAYLAND_DISPLAY={:?}, DISPLAY={:?}, XDG_CURRENT_DESKTOP={:?}, prgname={:?}",
            detected,
            session_type,
            wayland_display,
            x11_display,
            desktop,
            prg_name
        );
        crate::log_info!("🧭 App version: {}", env!("CARGO_PKG_VERSION"));
        if let Some(distro_name) = read_linux_distribution_name() {
            crate::log_info!("🧭 Linux distro: {}", distro_name);
        }

        if is_wayland_session() {
            let state = app.state::<AppState>();
            let host_app_registration = tauri::async_runtime::block_on(async {
                let app_id = AppID::try_from("org.voquill.foss")
                    .map_err(|error| format!("Invalid host app id: {error}"))?;
                register_host_app(app_id)
                    .await
                    .map_err(|error| format!("Failed to register host app with portal: {error}"))
            });

            match host_app_registration {
                Ok(()) => {
                    let mut registration_error =
                        state.wayland_host_app_registration_error.lock().unwrap();
                    *registration_error = None;
                    crate::log_info!("✅ Registered host app ID with portal registry");
                }
                Err(error) => {
                    let mut registration_error =
                        state.wayland_host_app_registration_error.lock().unwrap();
                    *registration_error = Some(error.clone());
                    crate::log_warn!("⚠️ Host app registration failed: {}", error);
                }
            }

            let tray_watcher_available =
                tauri::async_runtime::block_on(is_status_notifier_watcher_available());
            crate::log_info!(
                "🧭 StatusNotifier watcher available: {}",
                tray_watcher_available
            );
        }
    }

    if let Some(window) = app.get_webview_window("overlay") {
        crate::log_info!("🔍 Overlay window found in setup");
        let _ = window.hide();
        let state = app.state::<AppState>();
        state.display_backend.apply_overlay_hints(&window);
    } else {
        crate::log_info!("❌ Overlay window NOT FOUND in setup!");
    }
    let _ = audio::get_input_devices();

    let menu = create_tray_menu(app.handle())?;
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
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
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

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
    }

    let hotkey_string = initial_config.hotkey.clone();
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) = re_register_hotkey(&app_handle, &hotkey_string).await {
            crate::log_warn!("Initial hotkey registration failed: {}", error);
            let state = app_handle.state::<AppState>();
            let mut hotkey_error = state.hotkey_error.lock().unwrap();
            *hotkey_error = Some(error);
        }
    });

    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) =
                    crate::platform::linux::wayland::input::establish_input_session(&app_handle, false)
                        .await
                {
                    crate::log_warn!("Wayland input session restore failed: {}", error);
                }
            });
        }
    }

    Ok(())
}
