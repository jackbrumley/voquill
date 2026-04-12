// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Logging macro with timestamps
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        {
            let __timestamp = ::chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            let __message = format!($($arg)*);
            println!("[{}] {}", __timestamp, __message);
            crate::append_session_log("INFO", &__timestamp, &__message);
        }
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        {
            let __timestamp = ::chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
            let __message = format!($($arg)*);
            eprintln!("[{}] WARNING: {}", __timestamp, __message);
            crate::append_session_log("WARN", &__timestamp, &__message);
        }
    };
}

use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WebviewWindow,
};

// Global app handle for emitting events - using OnceLock for thread safety
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static SESSION_LOG_FILE: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
static SESSION_LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

fn get_session_log_path() -> Result<PathBuf, String> {
    let debug_dir = dirs::config_dir()
        .ok_or_else(|| "Could not find config directory".to_string())?
        .join("foss-voquill")
        .join("debug");
    fs::create_dir_all(&debug_dir).map_err(|error| error.to_string())?;
    Ok(debug_dir.join("session.log"))
}

fn get_app_config_root_dir() -> Result<PathBuf, String> {
    let root_dir = dirs::config_dir()
        .ok_or_else(|| "Could not find config directory".to_string())?
        .join("foss-voquill");

    fs::create_dir_all(&root_dir).map_err(|error| error.to_string())?;
    Ok(root_dir)
}

fn clear_directory_contents(
    path: &std::path::Path,
    preserve_filenames: &[&str],
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let entry_path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if preserve_filenames.iter().any(|name| *name == file_name) {
            continue;
        }

        if entry_path.is_dir() {
            fs::remove_dir_all(&entry_path).map_err(|error| error.to_string())?;
        } else {
            fs::remove_file(&entry_path).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn truncate_session_log_with_header() -> Result<(), String> {
    if let Some(lock) = SESSION_LOG_FILE.get() {
        if let Ok(mut maybe_file) = lock.lock() {
            if let Some(file) = maybe_file.as_mut() {
                file.set_len(0).map_err(|error| error.to_string())?;
                file.seek(SeekFrom::Start(0))
                    .map_err(|error| error.to_string())?;
                writeln!(
                    file,
                    "[{}] SESSION RESET | version={}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    env!("CARGO_PKG_VERSION")
                )
                .map_err(|error| error.to_string())?;
                return Ok(());
            }
        }
    }

    Err("Session log file handle unavailable".to_string())
}

fn initialize_session_logging() {
    let log_path = match get_session_log_path() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Failed to initialize session log path: {}", error);
            return;
        }
    };

    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
    {
        Ok(mut file) => {
            let startup_header = format!(
                "[{}] SESSION START | version={}\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                env!("CARGO_PKG_VERSION")
            );
            let _ = file.write_all(startup_header.as_bytes());
            let _ = SESSION_LOG_PATH.set(log_path);
            let _ = SESSION_LOG_FILE.set(Mutex::new(Some(file)));
        }
        Err(error) => {
            eprintln!("Failed to open session log file: {}", error);
        }
    }
}

pub fn append_session_log(level: &str, timestamp: &str, message: &str) {
    if let Some(lock) = SESSION_LOG_FILE.get() {
        if let Ok(mut maybe_file) = lock.lock() {
            if let Some(file) = maybe_file.as_mut() {
                let _ = writeln!(file, "[{}] {}: {}", timestamp, level, message);
            }
        }
    }
}

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

mod audio;
mod config;
mod history;
mod hotkey;
mod local_whisper;
mod model_manager;
pub mod platform;
mod transcription;
mod typing;

#[cfg(target_os = "linux")]
use ashpd::{register_host_app, AppID};
use config::{Config, TranscriptionMode};
use hotkey::HardwareHotkey;
#[cfg(target_os = "linux")]
use platform::linux::detection::is_wayland_session;
#[cfg(target_os = "linux")]
use platform::linux::wayland::env::{check_wayland_display, configure_linux_session_environment};
#[cfg(target_os = "linux")]
use platform::linux::wayland::portal::capabilities::PortalDiagnostics;
use platform::permissions::LinuxPermissions;

#[cfg(not(target_os = "linux"))]
#[derive(Clone, Debug, Serialize)]
pub struct PortalDiagnostics {
    pub available: bool,
    pub version: u32,
    pub supports_configure_shortcuts: bool,
    pub has_record_shortcut: bool,
    pub active_trigger: Option<String>,
    pub status: String,
    pub detail: Option<String>,
}

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub is_recording: Arc<Mutex<bool>>,
    pub is_mic_test_active: Arc<Mutex<bool>>,
    pub is_configuring_hotkey: Arc<Mutex<bool>>,
    pub hotkey_error: Arc<Mutex<Option<String>>>,
    pub hotkey_binding_state: Arc<Mutex<HotkeyBindingState>>,
    pub setup_status: Arc<Mutex<Option<String>>>,
    pub hardware_hotkey: Arc<Mutex<HardwareHotkey>>,
    pub cached_device: Arc<Mutex<Option<cpal::Device>>>,
    pub playback_stream: Arc<Mutex<Option<cpal::Stream>>>,
    pub mic_test_samples: Arc<Mutex<Vec<f32>>>,
    pub audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
    #[cfg(target_os = "linux")]
    pub hotkey_engine_cancel: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    #[cfg(target_os = "linux")]
    pub wayland_input_sender:
        Arc<Mutex<Option<platform::linux::wayland::input::WaylandTypeSender>>>,
    #[cfg(target_os = "linux")]
    pub wayland_input_cancel: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    #[cfg(target_os = "linux")]
    pub wayland_input_ready: Arc<Mutex<bool>>,
    #[cfg(target_os = "linux")]
    pub wayland_host_app_registration_error: Arc<Mutex<Option<String>>>,
    pub display_backend: Arc<dyn platform::traits::DisplayBackend>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HotkeyBindingState {
    pub bound: bool,
    pub listening: bool,
    pub detail: Option<String>,
    pub active_trigger: Option<String>,
}

impl Default for HotkeyBindingState {
    fn default() -> Self {
        Self {
            bound: false,
            listening: false,
            detail: None,
            active_trigger: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: Arc::new(Mutex::new(Config::default())),
            is_recording: Arc::new(Mutex::new(false)),
            is_mic_test_active: Arc::new(Mutex::new(false)),
            is_configuring_hotkey: Arc::new(Mutex::new(false)),
            hotkey_error: Arc::new(Mutex::new(None)),
            hotkey_binding_state: Arc::new(Mutex::new(HotkeyBindingState::default())),
            setup_status: Arc::new(Mutex::new(None)),
            hardware_hotkey: Arc::new(Mutex::new(HardwareHotkey::default())),
            cached_device: Arc::new(Mutex::new(None)),
            playback_stream: Arc::new(Mutex::new(None)),
            mic_test_samples: Arc::new(Mutex::new(Vec::new())),
            audio_engine: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            hotkey_engine_cancel: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            wayland_input_sender: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            wayland_input_cancel: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "linux")]
            wayland_input_ready: Arc::new(Mutex::new(false)),
            #[cfg(target_os = "linux")]
            wayland_host_app_registration_error: Arc::new(Mutex::new(None)),
            display_backend: platform::initialize(),
        }
    }
}

#[cfg(target_os = "linux")]
fn enrich_wayland_shortcut_error(state: &AppState, error: String) -> String {
    if !is_wayland_session() {
        return error;
    }

    let looks_like_portal_rejection = error.contains("Portal request didn't succeed: Other")
        || error.contains("Portal rejected shortcut");
    if !looks_like_portal_rejection {
        return error;
    }

    let host_registration_error = state
        .wayland_host_app_registration_error
        .lock()
        .unwrap()
        .clone();

    if let Some(host_error) = host_registration_error {
        return format!(
            "{} Hint: Wayland portal app registration failed earlier ({}). This usually means the desktop environment cannot resolve Voquill's app metadata yet.",
            error, host_error
        );
    }

    error
}

pub fn set_hotkey_binding_state(
    app_handle: &tauri::AppHandle,
    bound: bool,
    listening: bool,
    detail: Option<String>,
    active_trigger: Option<String>,
) {
    let state = app_handle.state::<AppState>();
    {
        let mut binding_state = state.hotkey_binding_state.lock().unwrap();
        binding_state.bound = bound;
        binding_state.listening = listening;
        binding_state.detail = detail;
        binding_state.active_trigger = active_trigger;
    }
    let snapshot = {
        let binding_state = state.hotkey_binding_state.lock().unwrap();
        binding_state.clone()
    };
    let _ = app_handle.emit("hotkey-binding-state", snapshot);
}

#[tauri::command]
async fn get_wayland_portal_version() -> Result<u32, String> {
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
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
            return Ok(0);
        }
    }
    Ok(0)
}

#[tauri::command]
async fn get_portal_diagnostics() -> Result<PortalDiagnostics, String> {
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            return Ok(
                platform::linux::wayland::portal::capabilities::collect_global_shortcuts_diagnostics().await,
            );
        }
    }

    Ok(PortalDiagnostics {
        available: false,
        version: 0,
        supports_configure_shortcuts: false,
        has_record_shortcut: false,
        active_trigger: None,
        status: "unsupported".to_string(),
        detail: Some("Portal diagnostics are only available on Linux Wayland.".to_string()),
    })
}

#[derive(Serialize)]
struct SystemShortcutContext {
    distro: Option<String>,
    desktop: Option<String>,
    settings_path: String,
}

#[tauri::command]
async fn get_system_shortcut_context() -> Result<SystemShortcutContext, String> {
    #[cfg(target_os = "linux")]
    {
        let distro = read_linux_distribution_name();
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .ok()
            .and_then(|value| value.split(':').next().map(|segment| segment.to_string()));

        let settings_path = match desktop.as_deref() {
            Some(value) if value.eq_ignore_ascii_case("GNOME") => {
                "System Settings -> Apps -> Voquill -> Global Shortcuts".to_string()
            }
            Some(value) if value.eq_ignore_ascii_case("KDE") => {
                "System Settings -> Keyboard -> Shortcuts -> Voquill".to_string()
            }
            _ => "System Settings -> search for 'Voquill' or 'Keyboard Shortcuts'".to_string(),
        };

        return Ok(SystemShortcutContext {
            distro,
            desktop,
            settings_path,
        });
    }

    #[cfg(not(target_os = "linux"))]
    {
        Ok(SystemShortcutContext {
            distro: None,
            desktop: None,
            settings_path: "System Settings -> Keyboard Shortcuts".to_string(),
        })
    }
}

#[tauri::command]
async fn get_linux_setup_status(
    state: tauri::State<'_, AppState>,
) -> Result<LinuxPermissions, String> {
    log_info!("📡 Tauri Command: get_linux_setup_status invoked");
    let config = {
        let guard = state.config.lock().unwrap();
        guard.clone()
    };
    let mut permissions = state.display_backend.check_permissions(&config).await;
    let binding_state = state.hotkey_binding_state.lock().unwrap().clone();
    if binding_state.bound {
        permissions.shortcuts = true;
        permissions.shortcuts_status = "bound".to_string();
        permissions.shortcuts_detail = binding_state.detail;
    }
    #[cfg(target_os = "linux")]
    if is_wayland_session() {
        let input_ready = *state.wayland_input_ready.lock().unwrap();
        permissions.input_emulation = input_ready;
    }
    log_info!(
        "🧭 Setup readiness: audio={}, shortcuts={} (status={}), input_emulation={}, runtime_hotkey_bound={}, runtime_hotkey_listening={}",
        permissions.audio,
        permissions.shortcuts,
        permissions.shortcuts_status,
        permissions.input_emulation,
        binding_state.bound,
        binding_state.listening
    );
    Ok(permissions)
}

#[tauri::command]
async fn request_audio_permission() -> Result<(), String> {
    log_info!("📡 Tauri Command: request_audio_permission invoked");
    #[cfg(target_os = "linux")]
    {
        use ashpd::desktop::camera::Camera;

        // On Wayland, the app id is inferred via matching the desktop file name and WM class.
        // It relies on the environment or StartupWMClass in the desktop file.

        let camera = Camera::new().await.map_err(|e| {
            format!(
                "Audio Portal not available: {}. Is xdg-desktop-portal installed?",
                e
            )
        })?;
        camera
            .request_access()
            .await
            .map_err(|e| format!("Audio access denied: {}", e))?;
        return Ok(());
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(())
    }
}

#[tauri::command]
async fn request_input_permission(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    log_info!("📡 Tauri Command: request_input_permission invoked");
    #[cfg(target_os = "linux")]
    {
        if is_wayland_session() {
            platform::linux::wayland::input::establish_input_session(&app_handle, true).await?;
        } else {
            let _ = state;
        }
        return Ok(());
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = state;
        let _ = app_handle;
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

#[derive(Serialize)]
struct ConfigureHotkeyResult {
    outcome: String,
    detail: Option<String>,
}

async fn apply_hotkey_registration(
    new_hotkey: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    log_info!("Manual hotkey registration requested for: {}", new_hotkey);

    let previous_hotkey = {
        let config = state.config.lock().unwrap();
        config.hotkey.clone()
    };

    {
        let mut config = state.config.lock().unwrap();
        config.hotkey = new_hotkey.clone();
    }

    let backend = state.display_backend.clone();
    match backend.start_engine(app_handle.clone(), true).await {
        Ok(()) => {
            let save_result = {
                let config = state.config.lock().unwrap();
                crate::config::save_config(&config)
            };

            let save_error = save_result.err().map(|error| error.to_string());
            if let Some(save_error) = save_error {
                log_warn!(
                    "Failed to persist new hotkey '{}': {}. Restoring previous hotkey '{}'.",
                    new_hotkey,
                    save_error,
                    previous_hotkey
                );

                {
                    let mut config = state.config.lock().unwrap();
                    config.hotkey = previous_hotkey.clone();
                }

                let restore_error = backend.start_engine(app_handle.clone(), false).await.err();
                if let Some(error) = restore_error {
                    let enriched_error = {
                        #[cfg(target_os = "linux")]
                        {
                            enrich_wayland_shortcut_error(&state, error)
                        }
                        #[cfg(not(target_os = "linux"))]
                        {
                            error
                        }
                    };
                    set_hotkey_binding_state(
                        &app_handle,
                        false,
                        false,
                        Some(enriched_error.clone()),
                        None,
                    );
                    let mut hotkey_error = state.hotkey_error.lock().unwrap();
                    *hotkey_error = Some(enriched_error.clone());
                    return Err(format!(
                        "Failed to save hotkey change: {}. Also failed to restore previous hotkey: {}",
                        save_error, enriched_error
                    ));
                }

                set_hotkey_binding_state(&app_handle, true, true, None, None);
                let mut hotkey_error = state.hotkey_error.lock().unwrap();
                *hotkey_error = None;
                return Err(format!(
                    "Failed to save hotkey change: {}. Previous hotkey was restored.",
                    save_error
                ));
            }

            set_hotkey_binding_state(&app_handle, true, true, None, None);
            let mut error = state.hotkey_error.lock().unwrap();
            *error = None;
            Ok(())
        }
        Err(error) => {
            let registration_error = {
                #[cfg(target_os = "linux")]
                {
                    enrich_wayland_shortcut_error(&state, error)
                }
                #[cfg(not(target_os = "linux"))]
                {
                    error
                }
            };

            {
                let mut config = state.config.lock().unwrap();
                config.hotkey = previous_hotkey.clone();
            }

            if previous_hotkey == new_hotkey {
                set_hotkey_binding_state(
                    &app_handle,
                    false,
                    false,
                    Some(registration_error.clone()),
                    None,
                );
                let mut hotkey_error = state.hotkey_error.lock().unwrap();
                *hotkey_error = Some(registration_error.clone());
                return Err(registration_error);
            }

            let restore_result = backend.start_engine(app_handle.clone(), false).await;
            match restore_result {
                Ok(()) => {
                    set_hotkey_binding_state(&app_handle, true, true, None, None);
                    let mut hotkey_error = state.hotkey_error.lock().unwrap();
                    *hotkey_error = None;
                    Err(registration_error)
                }
                Err(restore_error) => {
                    let enriched_restore_error = {
                        #[cfg(target_os = "linux")]
                        {
                            enrich_wayland_shortcut_error(&state, restore_error)
                        }
                        #[cfg(not(target_os = "linux"))]
                        {
                            restore_error
                        }
                    };
                    set_hotkey_binding_state(
                        &app_handle,
                        false,
                        false,
                        Some(enriched_restore_error.clone()),
                        None,
                    );
                    let mut hotkey_error = state.hotkey_error.lock().unwrap();
                    *hotkey_error = Some(enriched_restore_error.clone());
                    Err(format!(
                        "{} Also failed to restore previous hotkey: {}",
                        registration_error, enriched_restore_error
                    ))
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[tauri::command]
async fn configure_hotkey(
    state: tauri::State<'_, AppState>,
) -> Result<ConfigureHotkeyResult, String> {
    if is_wayland_session() {
        let capabilities =
            platform::linux::wayland::portal::capabilities::detect_global_shortcuts_capabilities()
                .await?;

        if !capabilities.supports_configure_shortcuts {
            return Ok(ConfigureHotkeyResult {
                outcome: "system_managed".to_string(),
                detail: Some(
                    "This desktop manages shortcut changes in system settings.".to_string(),
                ),
            });
        }

        let hotkey = {
            let config = state.config.lock().unwrap();
            config.hotkey.clone()
        };

        let opened_system_configuration =
            platform::linux::wayland::shortcuts::try_open_linux_portal_shortcut_configuration(
                &hotkey,
            )
            .await?;

        if opened_system_configuration {
            return Ok(ConfigureHotkeyResult {
                outcome: "configured".to_string(),
                detail: Some("Opened system shortcut configuration.".to_string()),
            });
        }

        return Ok(ConfigureHotkeyResult {
            outcome: "system_managed".to_string(),
            detail: Some("Shortcut changes must be made in system settings.".to_string()),
        });
    }

    Ok(ConfigureHotkeyResult {
        outcome: "requires_in_app_capture".to_string(),
        detail: None,
    })
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
async fn configure_hotkey() -> Result<ConfigureHotkeyResult, String> {
    Ok(ConfigureHotkeyResult {
        outcome: "requires_in_app_capture".to_string(),
        detail: None,
    })
}

#[tauri::command]
async fn apply_captured_hotkey(
    new_hotkey: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<ConfigureHotkeyResult, String> {
    apply_hotkey_registration(new_hotkey, state, app_handle).await?;
    Ok(ConfigureHotkeyResult {
        outcome: "configured".to_string(),
        detail: None,
    })
}

#[tauri::command]
async fn manual_register_hotkey(
    new_hotkey: String,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    apply_hotkey_registration(new_hotkey, state, app_handle).await
}

#[tauri::command]
async fn get_hotkey_binding_state(
    state: tauri::State<'_, AppState>,
) -> Result<HotkeyBindingState, String> {
    let binding_state = state.hotkey_binding_state.lock().unwrap();
    Ok(binding_state.clone())
}

#[cfg(target_os = "linux")]
async fn is_status_notifier_watcher_available() -> bool {
    use zbus::names::BusName;

    let connection = match zbus::Connection::session().await {
        Ok(connection) => connection,
        Err(error) => {
            log_warn!("Failed to open session DBus for tray check: {}", error);
            return false;
        }
    };

    let proxy = match zbus::fdo::DBusProxy::new(&connection).await {
        Ok(proxy) => proxy,
        Err(error) => {
            log_warn!("Failed to create DBus proxy for tray check: {}", error);
            return false;
        }
    };

    let kde_watcher = match BusName::try_from("org.kde.StatusNotifierWatcher") {
        Ok(name) => name,
        Err(error) => {
            log_warn!("Invalid KDE watcher bus name: {}", error);
            return false;
        }
    };
    let freedesktop_watcher = match BusName::try_from("org.freedesktop.StatusNotifierWatcher") {
        Ok(name) => name,
        Err(error) => {
            log_warn!("Invalid freedesktop watcher bus name: {}", error);
            return false;
        }
    };

    match proxy.name_has_owner(kde_watcher).await {
        Ok(true) => true,
        Ok(false) => match proxy.name_has_owner(freedesktop_watcher).await {
            Ok(value) => value,
            Err(error) => {
                log_warn!("Failed to check freedesktop tray watcher: {}", error);
                false
            }
        },
        Err(error) => {
            log_warn!("Failed to check KDE tray watcher: {}", error);
            false
        }
    }
}

#[tauri::command]
async fn minimize_to_tray_or_taskbar(app_handle: tauri::AppHandle) -> Result<String, String> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    #[cfg(target_os = "linux")]
    {
        if is_status_notifier_watcher_available().await {
            window.hide().map_err(|error| error.to_string())?;
            return Ok("tray".to_string());
        }

        window.minimize().map_err(|error| error.to_string())?;
        return Ok("taskbar".to_string());
    }

    #[cfg(not(target_os = "linux"))]
    {
        window.minimize().map_err(|error| error.to_string())?;
        Ok("taskbar".to_string())
    }
}

#[tauri::command]
async fn quit_application(app_handle: tauri::AppHandle) -> Result<(), String> {
    app_handle.exit(0);
    Ok(())
}

#[tauri::command]
async fn get_audio_devices() -> Result<Vec<audio::AudioDevice>, String> {
    log_info!("📡 Tauri Command: get_audio_devices invoked");
    audio::get_input_devices()
}

#[cfg(target_os = "linux")]
#[tauri::command]
async fn set_configuring_hotkey(
    is_configuring: bool,
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut config_flag = state.is_configuring_hotkey.lock().unwrap();
        *config_flag = is_configuring;
    }
    log_info!("🔧 set_configuring_hotkey: {}", is_configuring);

    if is_wayland_session() {
        if is_configuring {
            let cancelled = {
                let mut cancel_lock = state.hotkey_engine_cancel.lock().unwrap();
                if let Some(sender) = cancel_lock.take() {
                    let _ = sender.send(());
                    true
                } else {
                    false
                }
            };
            if cancelled {
                log_info!("⏸️  Paused Wayland hotkey engine while configuring shortcut");
            }
        } else {
            let should_resume = {
                let binding_state = state.hotkey_binding_state.lock().unwrap();
                !(binding_state.bound && binding_state.listening)
            };

            if should_resume {
                let backend = state.display_backend.clone();
                if let Err(error) = backend.start_engine(app_handle.clone(), false).await {
                    log_warn!(
                        "Failed to resume Wayland hotkey engine after configuration: {}",
                        error
                    );
                }
            } else {
                log_info!("▶️ Wayland hotkey engine already active after capture; skipping resume");
            }
        }
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
#[tauri::command]
async fn set_configuring_hotkey(
    is_configuring: bool,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut config_flag = state.is_configuring_hotkey.lock().unwrap();
        *config_flag = is_configuring;
    }
    log_info!("🔧 set_configuring_hotkey: {}", is_configuring);
    Ok(())
}

#[tauri::command]
async fn start_recording(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let recording_before = *state.is_recording.lock().unwrap();
    log_info!(
        "🎤 start_recording invoked: is_recording_before={}, configuring_hotkey={}",
        recording_before,
        *state.is_configuring_hotkey.lock().unwrap()
    );

    if *state.is_configuring_hotkey.lock().unwrap() {
        log_info!("⚠️ Ignoring start_recording because hotkey configuration is active");
        return Err("Currently configuring hotkey".to_string());
    }

    let mut recording_flag = state.is_recording.lock().unwrap();
    if *recording_flag {
        return Err("Already recording".to_string());
    }

    *recording_flag = true;
    log_info!(
        "🎤 start_recording command - Flag set true (before={}, after={})",
        recording_before,
        *recording_flag
    );

    let is_recording_clone = state.is_recording.clone();
    let config = state.config.clone();
    let app_handle_clone = app_handle.clone();
    let audio_engine = state.audio_engine.clone();

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
        let result =
            record_and_transcribe(config, is_recording_clone, app_handle_clone, audio_engine).await;

        if let Err(e) = result {
            log_info!("❌ Global Recording error: {}", e);
        }
    });

    Ok(())
}

#[tauri::command]
async fn stop_recording(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut recording = state.is_recording.lock().unwrap();
    let before = *recording;
    *recording = false;
    log_info!(
        "⏹️  stop_recording command - Flag set false (before={}, after={})",
        before,
        *recording
    );
    Ok(())
}

#[tauri::command]
async fn start_mic_test(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
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

        if engine_guard.is_none() {
            *mic_test_flag = false;
            return Err("Audio engine not initialized".to_string());
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
        })
        .await;

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

        let mut mic_test_flag = is_mic_test_clone.lock().unwrap();
        if *mic_test_flag {
            *mic_test_flag = false;
            log_info!("🔧 Mic test active flag reset after mic test completion");
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
        .join("foss-voquill")
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
async fn get_session_log_text() -> Result<String, String> {
    let log_path = SESSION_LOG_PATH
        .get()
        .cloned()
        .or_else(|| get_session_log_path().ok())
        .ok_or_else(|| "Session log path unavailable".to_string())?;

    std::fs::read_to_string(&log_path).map_err(|error| error.to_string())
}

#[tauri::command]
async fn copy_session_log_to_clipboard() -> Result<(), String> {
    let logs = get_session_log_text().await?;
    typing::copy_to_clipboard(&logs).map_err(|error| error.to_string())
}

#[tauri::command]
async fn open_session_log() -> Result<(), String> {
    let log_path = SESSION_LOG_PATH
        .get()
        .cloned()
        .or_else(|| get_session_log_path().ok())
        .ok_or_else(|| "Session log path unavailable".to_string())?;

    #[cfg(target_os = "linux")]
    {
        log_info!("🚀 Executing: xdg-open {:?}", log_path);
        std::process::Command::new("xdg-open")
            .arg(&log_path)
            .spawn()
            .map_err(|error| {
                log_info!("❌ Failed to execute xdg-open for session log: {}", error);
                error.to_string()
            })?;
    }

    #[cfg(target_os = "windows")]
    {
        log_info!("🚀 Executing: explorer {:?}", log_path);
        std::process::Command::new("explorer")
            .arg(&log_path)
            .spawn()
            .map_err(|error| {
                log_info!("❌ Failed to execute explorer for session log: {}", error);
                error.to_string()
            })?;
    }

    #[cfg(target_os = "macos")]
    {
        log_info!("🚀 Executing: open {:?}", log_path);
        std::process::Command::new("open")
            .arg(&log_path)
            .spawn()
            .map_err(|error| {
                log_info!("❌ Failed to execute open for session log: {}", error);
                error.to_string()
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
    let mut normalized_config = new_config;
    normalized_config.normalize_input_sensitivity();

    let (restart_engine, hotkey_changed) = {
        let mut config = state.config.lock().unwrap();
        let audio_changed = config.audio_device != normalized_config.audio_device
            || config.input_sensitivity != normalized_config.input_sensitivity;
        let hotkey_changed = config.hotkey != normalized_config.hotkey;

        // CRITICAL: Preserve internal tokens that the frontend doesn't manage
        let mut merged_config = normalized_config.clone();
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

    let is_mic_test_active = *state.is_mic_test_active.lock().unwrap();
    if restart_engine && !is_mic_test_active {
        log_info!("🔧 Audio config changed, restarting persistent engine...");
        let cached_device = state.cached_device.lock().unwrap().clone();
        let sensitivity = normalized_config.input_sensitivity;
        let mut engine_guard = state.audio_engine.lock().unwrap();
        *engine_guard = None; // Drop old stream
        if let Some(dev) = cached_device {
            if let Ok(new_eng) = audio::PersistentAudioEngine::new(&dev, sensitivity) {
                *engine_guard = Some(new_eng);
                log_info!("✅ Persistent engine restarted");
            }
        }
    } else if restart_engine {
        log_info!("🔧 Audio config changed during active mic test, deferring engine restart");
    }

    if let Err(e) = config::save_config(&normalized_config) {
        let error_msg = format!("Failed to save config: {}", e);
        return Err(error_msg);
    }

    if hotkey_changed {
        if let Err(e) = re_register_hotkey(&app_handle, &normalized_config.hotkey).await {
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

async fn re_register_hotkey(
    app_handle: &tauri::AppHandle,
    hotkey_string: &str,
) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    log_info!("🔄 Re-registering hotkey '{}'...", hotkey_string);

    let backend = state.display_backend.clone();
    match backend.start_engine(app_handle.clone(), false).await {
        Ok(()) => {
            set_hotkey_binding_state(app_handle, true, true, None, None);
            Ok(())
        }
        Err(error) => {
            set_hotkey_binding_state(app_handle, false, false, Some(error.clone()), None);
            Err(error)
        }
    }
}

#[tauri::command]
async fn test_api_key(api_key: String, api_url: String) -> Result<bool, String> {
    transcription::test_api_key(&api_key, &api_url)
        .await
        .map_err(|e| e.to_string())
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
    })
    .await?;

    Ok(())
}

static CURRENT_STATUS: OnceLock<Mutex<String>> = OnceLock::new();
static STATUS_UPDATE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize)]
struct StatusUpdatePayload {
    seq: u64,
    status: String,
}

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

#[tauri::command]
async fn reset_application_to_defaults(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    log_info!("🧹 Factory reset requested");

    let root_dir = get_app_config_root_dir()?;

    let models_dir = root_dir.join("models");
    if models_dir.exists() {
        fs::remove_dir_all(&models_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&models_dir).map_err(|error| error.to_string())?;

    let debug_dir = root_dir.join("debug");
    fs::create_dir_all(&debug_dir).map_err(|error| error.to_string())?;
    clear_directory_contents(&debug_dir, &["session.log"])?;

    if let Err(error) = truncate_session_log_with_header() {
        log_warn!(
            "⚠️ Could not truncate session log during factory reset: {}",
            error
        );
    }

    history::clear_history().map_err(|error| error.to_string())?;

    let default_config = Config::default();
    config::save_config(&default_config).map_err(|error| error.to_string())?;

    {
        let mut config_lock = state.config.lock().unwrap();
        *config_lock = default_config.clone();
    }

    {
        let mut cached_device = state.cached_device.lock().unwrap();
        *cached_device = None;
    }

    {
        let mut mic_test_active = state.is_mic_test_active.lock().unwrap();
        *mic_test_active = false;
    }

    {
        let mut playback_stream = state.playback_stream.lock().unwrap();
        *playback_stream = None;
    }

    {
        let mut hotkey_error = state.hotkey_error.lock().unwrap();
        *hotkey_error = None;
    }

    if let Err(error) = re_register_hotkey(&app_handle, &default_config.hotkey).await {
        let mut hotkey_error = state.hotkey_error.lock().unwrap();
        *hotkey_error = Some(error.clone());
        return Err(format!(
            "Factory reset completed but failed to re-register default hotkey: {}",
            error
        ));
    }

    emit_status_to_frontend("Ready").await;

    let _ = app_handle.emit("history-updated", serde_json::json!({ "items": [] }));
    let _ = app_handle.emit("config-updated", default_config.clone());
    let _ = app_handle.emit("setup-status-changed", serde_json::json!({}));

    log_info!("✅ Factory reset completed successfully");
    Ok(())
}

async fn hide_overlay_window(app_handle: &AppHandle) -> Result<(), String> {
    if let Some(overlay_window) = app_handle.get_webview_window("overlay") {
        overlay_window.hide().map_err(|e| e.to_string())?;
    } else {
        log_warn!("⚠️ hide_overlay_window: overlay window not found");
    }
    Ok(())
}

async fn position_overlay_window(
    overlay_window: &WebviewWindow,
    app_handle: &AppHandle,
) -> Result<(), String> {
    let app_state = app_handle.state::<AppState>();
    let pixels_from_bottom_logical = {
        let config = app_state.config.lock().unwrap();
        config.pixels_from_bottom
    };

    app_state
        .display_backend
        .position_overlay_window(overlay_window, pixels_from_bottom_logical)?;
    Ok(())
}

async fn show_overlay_window(app_handle: &AppHandle) -> Result<(), String> {
    let overlay_window = app_handle
        .get_webview_window("overlay")
        .ok_or("Overlay window not found")?;

    if overlay_window.is_visible().unwrap_or(false) {
        return Ok(());
    }

    position_overlay_window(&overlay_window, app_handle).await?;

    // Use Tauri native show() to maintain reference count stability
    overlay_window.show().map_err(|e| e.to_string())?;

    // Ghost Mode handled by display backend
    let state = app_handle.state::<AppState>();
    state.display_backend.apply_overlay_hints(&overlay_window);
    Ok(())
}

// Centralized status emitter
async fn emit_status_update(status: &str) {
    let sequence = STATUS_UPDATE_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut previous_status: Option<String> = None;
    let mut changed = false;
    if let Some(status_mutex) = CURRENT_STATUS.get() {
        if let Ok(mut global_status) = status_mutex.lock() {
            previous_status = Some(global_status.clone());
            if *global_status != status {
                *global_status = status.to_string();
                changed = true;
            }
        }
    }

    if !changed {
        return;
    }

    log_info!(
        "🔄 App Status Change: '{}' -> '{}'",
        previous_status.as_deref().unwrap_or("<unknown>"),
        status
    );

    if let Some(app_handle) = APP_HANDLE.get() {
        let windows = ["main", "overlay"];
        let payload = StatusUpdatePayload {
            seq: sequence,
            status: status.to_string(),
        };
        for window_label in &windows {
            if let Some(window) = app_handle.get_webview_window(window_label) {
                let _ = window.emit("status-update", payload.clone());
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

fn validate_audio_duration(
    audio_data: &[u8],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if audio_data.len() < 44 {
        return Err("Audio file too small".into());
    }
    let sample_rate = u32::from_le_bytes([
        audio_data[24],
        audio_data[25],
        audio_data[26],
        audio_data[27],
    ]);
    let channels = u16::from_le_bytes([audio_data[22], audio_data[23]]);
    let bits_per_sample = u16::from_le_bytes([audio_data[34], audio_data[35]]);

    let mut data_size = 0u32;
    let mut pos = 36;
    while pos + 8 <= audio_data.len() {
        let chunk_id = &audio_data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes([
            audio_data[pos + 4],
            audio_data[pos + 5],
            audio_data[pos + 6],
            audio_data[pos + 7],
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
        return Err("No data chunk".into());
    }
    let bytes_per_sample = (bits_per_sample / 8) as u32;
    let bytes_per_second = sample_rate * channels as u32 * bytes_per_sample;
    let duration_seconds = data_size as f64 / bytes_per_second as f64;

    log_info!("Audio duration: {:.3}s", duration_seconds);
    if duration_seconds < 0.1 {
        return Err("Audio too short".into());
    }
    Ok(())
}

async fn record_and_transcribe(
    config: Arc<Mutex<Config>>,
    is_recording: Arc<Mutex<bool>>,
    app_handle: AppHandle,
    audio_engine: Arc<Mutex<Option<audio::PersistentAudioEngine>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let reset_status_on_exit = || async {
        emit_status_to_frontend("Ready").await;
    };

    let audio_data = match audio::record_audio_while_flag(&is_recording, audio_engine).await {
        Ok(data) => data,
        Err(e) => {
            reset_status_on_exit().await;
            return Err(e);
        }
    };

    if audio_data.is_empty() {
        reset_status_on_exit().await;
        return Ok(());
    }
    if let Err(e) = validate_audio_duration(&audio_data) {
        log_info!("⚠️ Audio validation failed: {}", e);
        reset_status_on_exit().await;
        return Ok(());
    }

    emit_status_to_frontend("Transcribing").await;
    let (
        transcription_mode,
        api_key,
        api_url,
        api_model,
        debug_mode,
        enable_recording_logs,
        language_choice,
    ) = {
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
            .join("foss-voquill")
            .join("debug")
            .join(format!(
                "recording_{}.wav",
                ::chrono::Local::now().format("%Y%m%d_%H%M%S")
            ));

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

    let service: Box<dyn transcription::TranscriptionService + Send + Sync> =
        match transcription_mode {
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

    let text = match service
        .transcribe(&audio_data, lang_code, prompt_hint)
        .await
    {
        Ok(text) => {
            log_info!(
                "📝 Transcription received ({}): \"{}\"",
                service.service_name(),
                text
            );
            text
        }

        Err(e) => {
            log_info!(
                "❌ Transcription failed ({}): {}",
                service.service_name(),
                e
            );
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
                config_guard.copy_on_typewriter,
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
                if let Err(e) = state
                    .display_backend
                    .type_text_hardware(&app_handle, &text, typing_speed, hold_duration)
                    .await
                {
                    log_info!("❌ TYPING ENGINE ERROR: {}", e);
                }
            }
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
    initialize_session_logging();

    #[cfg(target_os = "linux")]
    {
        configure_linux_session_environment();
    }

    env_logger::init();

    let _is_first_launch = config::is_first_launch().unwrap_or(false);
    let initial_config = config::load_config().unwrap_or_default();

    let app_state = AppState {
        config: Arc::new(Mutex::new(initial_config.clone())),
        hardware_hotkey: Arc::new(Mutex::new(hotkey::parse_hardware_hotkey(
            &initial_config.hotkey,
        ))),
        ..Default::default()
    };

    {
        let mut cached_device = app_state.cached_device.lock().unwrap();
        let dev = audio::lookup_device(initial_config.audio_device.clone()).ok();
        *cached_device = dev.clone();

        if let Some(d) = dev {
            if let Ok(engine) =
                audio::PersistentAudioEngine::new(&d, initial_config.input_sensitivity)
            {
                let mut engine_guard = app_state.audio_engine.lock().unwrap();
                *engine_guard = Some(engine);
                log_info!("✅ Persistent audio engine initialized");
            }
        }
        log_info!("🔧 Initial pre-warm of audio device cache complete");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
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
                let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
                let x11_display = std::env::var("DISPLAY").ok();
                let session_type = std::env::var("XDG_SESSION_TYPE").ok();
                let desktop = std::env::var("XDG_CURRENT_DESKTOP").ok();
                let prg_name = gtk::glib::prgname();
                let detected = crate::platform::linux::detection::detect_display_server();
                log_info!(
                    "🧭 Launch context: detected={:?}, XDG_SESSION_TYPE={:?}, WAYLAND_DISPLAY={:?}, DISPLAY={:?}, XDG_CURRENT_DESKTOP={:?}, prgname={:?}",
                    detected,
                    session_type,
                    wayland_display,
                    x11_display,
                    desktop,
                    prg_name
                );
                log_info!("🧭 App version: {}", env!("CARGO_PKG_VERSION"));
                if let Some(distro_name) = read_linux_distribution_name() {
                    log_info!("🧭 Linux distro: {}", distro_name);
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
                            log_info!("✅ Registered host app ID with portal registry");
                        }
                        Err(error) => {
                            let mut registration_error =
                                state.wayland_host_app_registration_error.lock().unwrap();
                            *registration_error = Some(error.clone());
                            log_warn!("⚠️ Host app registration failed: {}", error);
                        }
                    }

                    let tray_watcher_available =
                        tauri::async_runtime::block_on(is_status_notifier_watcher_available());
                    log_info!(
                        "🧭 StatusNotifier watcher available: {}",
                        tray_watcher_available
                    );
                }
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
                if let Err(error) = re_register_hotkey(&app_handle, &hotkey_string).await {
                    log_warn!("Initial hotkey registration failed: {}", error);
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
                            platform::linux::wayland::input::establish_input_session(&app_handle, false)
                                .await
                        {
                            log_warn!("Wayland input session restore failed: {}", error);
                        }
                    });
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording, stop_recording, get_config, save_config, reset_application_to_defaults,
            test_api_key, get_current_status, get_history, clear_history,
            check_hotkey_status, manual_register_hotkey, configure_hotkey, apply_captured_hotkey,
            get_hotkey_binding_state, minimize_to_tray_or_taskbar, quit_application, get_audio_devices,
            start_mic_test, stop_mic_test, stop_mic_playback, open_debug_folder, get_session_log_text,
            copy_session_log_to_clipboard, open_session_log,
            log_ui_event, get_available_engines, get_available_models, check_model_status, download_model,
            get_linux_setup_status, request_audio_permission, request_input_permission, set_configuring_hotkey,
            get_wayland_portal_version, get_portal_diagnostics, get_system_shortcut_context
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn create_tray_menu(app: &tauri::AppHandle) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let open_i = MenuItem::with_id(app, "open", "Open Voquill", true, None::<&str>)?;
    Menu::with_items(app, &[&open_i, &quit_i])
}
