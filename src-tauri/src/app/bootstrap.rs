use crate::app::commands::hotkey::re_register_hotkey;
use crate::app::session_log;
use crate::app::state::AppState;
use crate::app::tray;
use crate::audio;
use crate::config::storage::{ConfigRecovery, LoadedConfig};
use crate::config::Config;
use crate::engine_factory;
#[cfg(target_os = "linux")]
use ashpd::{register_host_app, AppID};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

#[cfg(target_os = "linux")]
use crate::platform::linux::detection::is_wayland_session;
#[cfg(target_os = "linux")]
use crate::platform::linux::wayland::env::{check_wayland_display, APP_ID};

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

/// Records how Linux launched Voquill (display server, session, desktop,
/// distro) so every session log carries the environment it ran in.
#[cfg(target_os = "linux")]
fn log_linux_launch_context() {
    crate::log_info!(
        "Launch context: detected={:?}, XDG_SESSION_TYPE={:?}, WAYLAND_DISPLAY={:?}, DISPLAY={:?}, XDG_CURRENT_DESKTOP={:?}, prgname={:?}",
        crate::platform::linux::detection::detect_display_server(),
        std::env::var("XDG_SESSION_TYPE").ok(),
        std::env::var("WAYLAND_DISPLAY").ok(),
        std::env::var("DISPLAY").ok(),
        std::env::var("XDG_CURRENT_DESKTOP").ok(),
        gtk::glib::prgname()
    );
    if let Some(distro_name) = read_linux_distribution_name() {
        crate::log_info!("Linux distro: {}", distro_name);
    }
}

/// Registers Voquill's app ID with the portal host registry, recording any
/// failure for the setup status the UI shows.
#[cfg(target_os = "linux")]
fn register_wayland_host_app(state: &AppState) {
    let registration = tauri::async_runtime::block_on(async {
        let app_id =
            AppID::try_from(APP_ID).map_err(|error| format!("Invalid host app id: {error}"))?;
        register_host_app(app_id)
            .await
            .map_err(|error| format!("Failed to register host app with portal: {error}"))
    });

    let mut registration_error = state.wayland_host_app_registration_error.lock().unwrap();
    match registration {
        Ok(()) => {
            *registration_error = None;
            crate::log_info!("Registered host app ID with portal registry");
        }
        Err(error) => {
            crate::log_warn!("Host app registration failed: {}", error);
            *registration_error = Some(error);
        }
    }
}

/// Logs detected CPU SIMD capabilities to the session log so crash reports
/// include the reporter's actual hardware feature set (e.g. diagnosing
/// illegal-instruction crashes from mismatched whisper.cpp builds).
#[cfg(target_arch = "x86_64")]
fn log_cpu_features() {
    crate::log_info!(
        "CPU features: sse42={} avx={} avx2={} fma={} f16c={} avx512f={}",
        std::arch::is_x86_feature_detected!("sse4.2"),
        std::arch::is_x86_feature_detected!("avx"),
        std::arch::is_x86_feature_detected!("avx2"),
        std::arch::is_x86_feature_detected!("fma"),
        std::arch::is_x86_feature_detected!("f16c"),
        std::arch::is_x86_feature_detected!("avx512f"),
    );
}

#[cfg(not(target_arch = "x86_64"))]
fn log_cpu_features() {}

/// The first startup work that touches the user's files: storage migration,
/// session logging and the settings load. Runs from setup, after the
/// single-instance guard has let only one instance through.
fn initialize_storage_and_logging() -> LoadedConfig {
    // Migrate legacy data location (%APPDATA%\foss-voquill etc.) to the
    // unified ~/.config/voquill-app root before anything touches storage.
    let storage_migration_report = crate::paths::migrate_legacy_location();

    // Clean up any legacy autostart entries from previous application versions.
    let autostart_cleanup_report = crate::paths::cleanup_legacy_autostart_entries();

    // Session logging is always enabled. The persistence toggle is available
    // for future use (e.g. a private mode setting).
    session_log::set_persistence_enabled(true);
    session_log::initialize_session_logging();
    session_log::install_panic_hook();

    if let Some(report) = storage_migration_report {
        crate::log_info!("Storage migration: {}", report);
    }
    for item in autostart_cleanup_report {
        crate::log_info!("Autostart migration: {}", item);
    }

    #[cfg(target_os = "macos")]
    if let Ok(manifest) = std::env::var("VK_ICD_FILENAMES") {
        crate::log_info!("Using bundled MoltenVK driver manifest: {}", manifest);
    }
    log_cpu_features();

    // Loaded after session logging starts so load errors and migrations are
    // recorded in the session log.
    load_initial_config()
}

/// Loads `config.json` for startup. A settings file that exists but cannot be
/// read stops the app: starting on defaults would overwrite the user's
/// settings on the next save. The error is shown in a native dialog because
/// the main window may be hidden (autostart launches hidden).
fn load_initial_config() -> LoadedConfig {
    match crate::config::storage::load_config() {
        Ok(loaded_config) => loaded_config,
        Err(error) => {
            crate::log_warn!("Startup aborted: {}", error);
            rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Voquill could not load your settings")
                .set_description(format!(
                    "{}\n\nVoquill has not started, so your settings file was left untouched. Close any program that may be using it and try again.",
                    error
                ))
                .set_buttons(rfd::MessageButtons::Ok)
                .show();
            std::process::exit(1);
        }
    }
}

/// Tells the user their unparseable settings file was backed up and defaults
/// are in use. Shown from setup (not before Tauri boots) so the dialog runs on
/// Tauri's own UI thread.
fn show_config_recovery_dialog(app: &tauri::App<tauri::Wry>, recovery: &ConfigRecovery) {
    app.dialog()
        .message(format!(
            "Your settings file could not be read, so Voquill has started with default settings.\n\nThe original file was kept at:\n{}\n\nReason: {}",
            recovery.backup_path.display(),
            recovery.parse_error
        ))
        .title("Voquill settings were reset")
        .kind(MessageDialogKind::Warning)
        .show(|_| {});
}

fn build_app_state(initial_config: &Config) -> AppState {
    let app_state = AppState {
        config: Arc::new(Mutex::new(initial_config.clone())),
        ..Default::default()
    };

    {
        let mut cached_device = app_state.cached_device.lock().unwrap();
        let device = match audio::lookup_device(initial_config.audio_device.clone()) {
            Ok(device) => Some(device),
            Err(error) => {
                crate::log_warn!(
                    "Initial audio device pre-warm failed (requested_device='{}'): {}",
                    initial_config
                        .audio_device
                        .clone()
                        .unwrap_or_else(|| "default".to_string()),
                    error
                );
                None
            }
        };
        *cached_device = device.clone();

        if let Some(device) = device {
            match audio::PersistentAudioEngine::new(&device, initial_config.input_sensitivity) {
                Ok(engine) => {
                    let mut engine_guard = app_state.audio_engine.lock().unwrap();
                    *engine_guard = Some(engine);
                    crate::log_info!("Persistent audio engine initialized");
                }
                Err(error) => {
                    crate::log_warn!(
                        "Initial persistent audio engine initialization failed (requested_device='{}', sensitivity={:.2}): {}",
                        initial_config
                            .audio_device
                            .clone()
                            .unwrap_or_else(|| "default".to_string()),
                        initial_config.input_sensitivity,
                        error
                    );
                }
            }
        }
        crate::log_info!("Initial pre-warm of audio device cache complete");
    }

    app_state
}

/// How this instance was launched (from the command line).
pub struct LaunchOptions {
    /// Autostart: stay in the tray.
    pub start_hidden: bool,
    /// The in-app updater relaunched this version because the install failed.
    pub update_failed: bool,
}

pub fn run_setup(
    app: &mut tauri::App<tauri::Wry>,
    launch: LaunchOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let LoadedConfig {
        config: initial_config,
        recovery: config_recovery,
    } = initialize_storage_and_logging();
    app.manage(build_app_state(&initial_config));

    crate::app::status::initialize(app.handle().clone());

    if let Some(recovery) = &config_recovery {
        show_config_recovery_dialog(app, recovery);
    }

    #[cfg(target_os = "linux")]
    {
        check_wayland_display();
        log_linux_launch_context();
        if is_wayland_session() {
            register_wayland_host_app(&app.state::<AppState>());
        }
    }

    if let Some(window) = app.get_webview_window("overlay") {
        crate::log_info!("Overlay window found in setup");
        let _ = window.hide();
        let state = app.state::<AppState>();
        state.display_backend.apply_overlay_hints(&window);
    } else {
        crate::log_info!("Overlay window NOT FOUND in setup!");
    }
    let _ = audio::get_input_devices();

    tray::build_tray(app)?;
    tray::intercept_main_window_close(app);

    let tray_available = tauri::async_runtime::block_on(tray::is_tray_available());
    crate::log_info!("System tray available: {}", tray_available);
    if launch.start_hidden && !tray_available {
        crate::log_warn!(
            "Start hidden requested but no system tray is available; showing the main window so the app stays reachable"
        );
    }
    if !launch.start_hidden || !tray_available {
        tray::show_main_window(app.handle());
    }
    if launch.update_failed {
        crate::app::commands::update::show_update_failed_dialog(app.handle());
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
                if let Err(error) = crate::platform::linux::wayland::input::establish_input_session(
                    &app_handle,
                    false,
                )
                .await
                {
                    crate::log_warn!("Wayland input session restore failed: {}", error);
                }
            });
        }
    }

    spawn_engine_preload(
        app.state::<AppState>().engine_factory.clone(),
        &initial_config,
    );
    spawn_post_process_warmup(
        app.state::<AppState>().post_process_factory.clone(),
        &initial_config,
        app.handle(),
    );
    spawn_python_runner_prewarm(app.handle(), &initial_config);
    crate::voice_macro::sync_voice_macro_listener(app.handle());

    Ok(())
}

/// Pre-loads the transcription engine model into its cache so the first
/// recording reuses a warm model instead of paying the full load cost.
/// Fired at startup and re-armed whenever the engine/model changes or a
/// matching model download completes.
pub fn spawn_engine_preload(factory: Arc<engine_factory::EngineFactory>, config: &Config) {
    let config = config.clone();
    tauri::async_runtime::spawn(async move {
        factory.preload(&config).await;
    });
}

/// Warms the local post-process sidecar when post-processing is enabled, so
/// the first dictation reuses a warm llama-server instead of paying the
/// process spawn + GGUF load cost mid-session. Fired at startup and re-armed
/// whenever post-process settings change or a matching model download
/// completes. Emits `post-process-gpu-status-changed` when finished so the
/// settings UI reflects the fresh GPU start attempt.
pub fn spawn_post_process_warmup(
    factory: Arc<crate::post_process::factory::PostProcessFactory>,
    config: &Config,
    app_handle: &tauri::AppHandle,
) {
    let config = config.clone();
    let app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        factory.preload(&config).await;
        let _ = app_handle.emit("post-process-gpu-status-changed", ());
    });
}

/// Pre-warms the Python runner for diarization when diarization is enabled.
/// Spawned asynchronously at startup so it's ready before the user's first
/// file import, avoiding the ~2s lazy-start delay on first use.
pub fn spawn_python_runner_prewarm(app_handle: &tauri::AppHandle, config: &Config) {
    if !config.diarization_enabled_files {
        return;
    }
    let app_handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        crate::log_info!("Pre-warming Python runner for diarization...");
        let state = app_handle.state::<AppState>();
        match state.get_or_start_python_runner(&app_handle).await {
            Ok(_) => {
                crate::log_info!("Python runner pre-warmed successfully");
            }
            Err(e) => {
                crate::log_warn!("Failed to pre-warm Python runner: {}", e);
            }
        }
    });
}
