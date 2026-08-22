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
            $crate::append_session_log("INFO", &__timestamp, &__message);
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
            $crate::append_session_log("WARN", &__timestamp, &__message);
        }
    };
}

#[cfg(not(target_os = "linux"))]
use serde::Serialize;
use tauri::Manager;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "voquill",
    version,
    about = "Cross-platform push-to-talk dictation app"
)]
struct Cli {
    /// Start hidden in the system tray
    #[arg(short, long)]
    start_hidden: bool,
}

mod app;
mod archive;
mod audio;
mod config;
mod diarization;
mod engine_factory;
mod history;
mod hotkey;
mod local_whisper;
mod model_manager;
mod parakeet;
mod paths;
pub mod platform;
mod post_process;
mod python_runner;
mod sidecar;
mod text_cleanup;
mod transcription;
mod typing;

pub use app::commands::hotkey::set_hotkey_binding_state;
use app::commands::*;
pub use app::session_log::append_session_log;
pub(crate) use app::session_log::{
    clear_directory_contents, get_app_config_root_dir, resolve_session_log_path,
    truncate_session_log_with_header,
};
pub use app::state::{AppState, HotkeyBindingState};
#[cfg(target_os = "linux")]
use platform::linux::wayland::env::configure_linux_session_environment;

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

/// Logs detected CPU SIMD capabilities to the session log so crash reports
/// include the reporter's actual hardware feature set (e.g. diagnosing
/// illegal-instruction crashes from mismatched whisper.cpp builds).
#[cfg(target_arch = "x86_64")]
fn log_cpu_features() {
    log_info!(
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

fn main() {
    let cli = Cli::parse();
    let start_hidden = cli.start_hidden;

    // Third-party Vulkan "implicit layers" (Steam overlay, OBS capture, NVIDIA
    // Optimus switching) get injected into every Vulkan process and can corrupt
    // vkEnumeratePhysicalDevices on hybrid-graphics machines, causing whisper.cpp
    // to hang indefinitely. Since we only use Vulkan for local compute (never
    // rendering), disabling all layers is safe. Must be set before any
    // Vulkan-touching code runs.
    // SAFETY: called as the first statement in main(), before other threads exist.
    unsafe {
        std::env::set_var("VK_LOADER_LAYERS_DISABLE", "~all~");
    }

    // Migrate legacy data location (%APPDATA%\foss-voquill etc.) to the
    // unified ~/.config/voquill-app root before anything touches storage.
    let storage_migration_report = paths::migrate_legacy_location();

    let initial_config = config::load_config().unwrap_or_default();

    // Session logging is always enabled. The persistence toggle is available
    // for future use (e.g. a private mode setting).
    app::session_log::set_persistence_enabled(true);
    app::session_log::initialize_session_logging();

    if let Some(report) = storage_migration_report {
        log_info!("Storage migration: {}", report);
    }

    log_cpu_features();

    #[cfg(target_os = "linux")]
    {
        configure_linux_session_environment();
    }

    env_logger::init();

    let app_state = app::bootstrap::build_app_state(&initial_config);

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    // Ignore plugin hotkeys on Wayland, use Portal instead
                    if std::env::var("WAYLAND_DISPLAY").is_ok() {
                        return;
                    }
                    let app_handle = app.clone();
                    let pressed =
                        event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed;
                    tauri::async_runtime::spawn(async move {
                        let state = app_handle.state::<AppState>();
                        if pressed {
                            app::hotkey_handler::handle_hotkey_press(state, app_handle.clone())
                                .await;
                        } else {
                            app::hotkey_handler::handle_hotkey_release(state).await;
                        }
                    });
                })
                .build(),
        )
        .manage(app_state)
        .setup(move |app| app::bootstrap::run_setup(app, &initial_config, start_hidden))
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            get_config,
            save_config,
            reset_application_to_defaults,
            test_api_key,
            get_current_status,
            get_history,
            search_history,
            clear_history,
            delete_history_item,
            get_history_audio,
            check_hotkey_status,
            manual_register_hotkey,
            configure_hotkey,
            apply_captured_hotkey,
            get_hotkey_binding_state,
            minimize_to_tray_or_taskbar,
            quit_application,
            get_audio_devices,
            get_output_devices,
            start_mic_test,
            stop_mic_test,
            stop_mic_playback,
            play_history_recording,
            stop_history_recording,
            open_debug_folder,
            clear_recording_logs,
            get_session_log_text,
            copy_session_log_to_clipboard,
            open_session_log,
            log_ui_event,
            get_available_engines,
            get_available_models,
            check_model_status,
            download_model,
            preload_transcription_engine,
            transcribe_audio_file,
            test_cleanup_api,
            get_linux_setup_status,
            request_audio_permission,
            request_input_permission,
            set_configuring_hotkey,
            get_wayland_portal_version,
            get_portal_diagnostics,
            get_system_shortcut_context,
            get_overlay_positioning_capabilities,
            check_for_updates,
            get_gpu_status,
            get_post_process_gpu_status,
            get_engine_capabilities,
            unload_model
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
