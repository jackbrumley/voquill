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

    /// Set by the in-app updater when it relaunches this version because the
    /// install failed
    #[arg(long, hide = true)]
    update_failed: bool,
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
pub mod process_guard;
mod python_runner;
mod sidecar;
mod text_cleanup;
mod transcription;
mod typing;
pub mod voice_macro;

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

#[cfg(target_os = "macos")]
fn configure_bundled_vulkan() {
    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    let Some(contents_dir) = executable.parent().and_then(std::path::Path::parent) else {
        return;
    };
    let manifest = contents_dir
        .join("Resources")
        .join("packaging")
        .join("macos")
        .join("MoltenVK_icd.json");
    if manifest.is_file() {
        // SAFETY: called before any Vulkan-dependent code or worker threads start.
        unsafe {
            std::env::set_var("VK_ICD_FILENAMES", &manifest);
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn configure_bundled_vulkan() {}

fn main() {
    let cli = Cli::parse();
    let launch = app::bootstrap::LaunchOptions {
        start_hidden: cli.start_hidden,
        update_failed: cli.update_failed,
    };

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
    configure_bundled_vulkan();

    #[cfg(target_os = "linux")]
    {
        configure_linux_session_environment();
    }

    env_logger::init();

    // Only process-local setup happens above. Everything that touches the
    // user's files, session log or devices runs in setup, after the
    // single-instance guard.
    tauri::Builder::default()
        // Must be the first plugin: a second launch hands its arguments to the
        // running instance and exits here, before it can rotate that
        // instance's session log, rewrite its settings or open its devices.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            app::tray::handle_second_launch(app, launch_shows_window(&args));
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("voquill")
                .arg("--start-hidden")
                .build(),
        )
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
        .setup(move |app| app::bootstrap::run_setup(app, launch))
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
            close_main_window,
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
            get_platform_setup_status,
            request_audio_permission,
            request_input_permission,
            set_configuring_hotkey,
            get_wayland_portal_version,
            get_system_shortcut_context,
            get_overlay_positioning_capabilities,
            get_input_capabilities,
            check_for_updates,
            install_update,
            get_gpu_status,
            get_post_process_gpu_status,
            get_engine_capabilities,
            unload_model,
            test_voice_macro_sound,
            test_voice_macro_execution,
            test_spoken_voice_macro,
            get_available_tts_voices,
            download_tts_voice_model,
            preview_tts_voice,
            save_macro_tts_audio,
            import_macro_audio_file,
            save_macro_mic_recording,
            play_macro_sound_preview,
            delete_macro_sound,
            clone_macro_sound,
            stop_macro_sound_playback,
            get_available_base_voice_models,
            preview_custom_tts_voice,
            get_custom_voice_presets,
            save_custom_voice_preset,
            delete_custom_voice_preset
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            // The single shutdown owner: every exit (tray Quit, update install,
            // OS logout) passes through here, so child processes and devices
            // are always released.
            tauri::RunEvent::Exit => {
                // Absent only when setup failed before managing the state.
                if let Some(state) = app_handle.try_state::<AppState>() {
                    state.cleanup();
                }
            }
            // Clicking the Dock icon while the window is hidden.
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => app::tray::show_main_window(app_handle),
            _ => {}
        });
}

/// Whether a launch should show the main window: every launch does, except
/// autostart's `--start-hidden`. Arguments that fail to parse still show it.
fn launch_shows_window(args: &[String]) -> bool {
    Cli::try_parse_from(args)
        .map(|cli| !cli.start_hidden)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::launch_shows_window;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn plain_launch_shows_window() {
        assert!(launch_shows_window(&args(&["voquill"])));
    }

    #[test]
    fn start_hidden_launch_keeps_window_hidden() {
        assert!(!launch_shows_window(&args(&["voquill", "--start-hidden"])));
        assert!(!launch_shows_window(&args(&["voquill", "-s"])));
    }

    #[test]
    fn unparseable_launch_shows_window() {
        assert!(launch_shows_window(&args(&["voquill", "--unknown-flag"])));
    }
}
